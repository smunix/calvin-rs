//! Network client for connecting to a remote Calvin server.
//!
//! This module implements a TCP client that connects to a Calvin server
//! and sends expressions for remote evaluation.

use super::{serialize_message, Message, NetError, PROTOCOL_VERSION};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};

/// A client for connecting to a remote Calvin server.
pub struct Client {
    stream: TcpStream,
    remote_host: String,
    next_id: AtomicU64,
}

impl Client {
    /// Connect to a Calvin server at the given address.
    pub fn connect(address: &str) -> Result<Self, NetError> {
        let stream = TcpStream::connect(address)?;
        let remote_host = address.to_string();

        let mut client = Client {
            stream,
            remote_host,
            next_id: AtomicU64::new(1),
        };

        // Perform handshake
        client.handshake()?;

        Ok(client)
    }

    /// Perform the initial handshake with the server.
    fn handshake(&mut self) -> Result<(), NetError> {
        let msg = Message::Handshake {
            version: PROTOCOL_VERSION,
            client_name: "calvin-client".to_string(),
        };
        self.send_message(&msg)?;
        let response = self.recv_message()?;
        match response {
            Message::HandshakeAck { version, .. } => {
                if version != PROTOCOL_VERSION {
                    return Err(NetError::Protocol(format!(
                        "Version mismatch: client={}, server={}",
                        PROTOCOL_VERSION, version
                    )));
                }
                Ok(())
            }
            _ => Err(NetError::Protocol("Expected HandshakeAck".to_string())),
        }
    }

    /// Evaluate an expression on the remote server.
    pub fn eval(&mut self, expression: &str) -> Result<String, NetError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = Message::EvalRequest {
            id,
            expression: expression.to_string(),
        };
        self.send_message(&msg)?;
        let response = self.recv_message()?;
        match response {
            Message::EvalResponse { result, .. } => {
                result.map_err(|e| NetError::Server(e))
            }
            _ => Err(NetError::Protocol("Expected EvalResponse".to_string())),
        }
    }

    /// Define a variable on the remote server.
    pub fn define(&mut self, name: &str, expression: &str) -> Result<(), NetError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = Message::DefineRequest {
            id,
            name: name.to_string(),
            expression: expression.to_string(),
        };
        self.send_message(&msg)?;
        let response = self.recv_message()?;
        match response {
            Message::DefineResponse { result, .. } => {
                result.map_err(|e| NetError::Server(e))
            }
            _ => Err(NetError::Protocol("Expected DefineResponse".to_string())),
        }
    }

    /// List all bindings on the remote server.
    pub fn list_bindings(&mut self) -> Result<Vec<(String, String)>, NetError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = Message::ListBindingsRequest { id };
        self.send_message(&msg)?;
        let response = self.recv_message()?;
        match response {
            Message::ListBindingsResponse { bindings, .. } => Ok(bindings),
            _ => Err(NetError::Protocol(
                "Expected ListBindingsResponse".to_string(),
            )),
        }
    }

    /// Send a ping to the server.
    pub fn ping(&mut self) -> Result<(), NetError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = Message::Ping { id };
        self.send_message(&msg)?;
        let response = self.recv_message()?;
        match response {
            Message::Pong { .. } => Ok(()),
            _ => Err(NetError::Protocol("Expected Pong".to_string())),
        }
    }

    /// Request the server to shut down.
    pub fn shutdown(&mut self) -> Result<(), NetError> {
        let msg = Message::Shutdown;
        self.send_message(&msg)?;
        Ok(())
    }

    /// Get the remote host address.
    pub fn remote_host(&self) -> &str {
        &self.remote_host
    }

    /// Send a message to the server.
    fn send_message(&mut self, msg: &Message) -> Result<(), NetError> {
        let bytes = serialize_message(msg)?;
        self.stream.write_all(&bytes)?;
        self.stream.flush()?;
        Ok(())
    }

    /// Receive a message from the server.
    fn recv_message(&mut self) -> Result<Message, NetError> {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut data = vec![0u8; len];
        self.stream.read_exact(&mut data)?;

        let json =
            std::str::from_utf8(&data).map_err(|e| NetError::Protocol(e.to_string()))?;
        let msg: Message =
            serde_json::from_str(json).map_err(|e| NetError::Serialization(e.to_string()))?;
        Ok(msg)
    }
}
