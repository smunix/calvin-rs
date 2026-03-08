//! Network server for remote expression evaluation.
//!
//! This module implements a TCP server that accepts connections and
//! evaluates Calvin expressions remotely, similar to hobbes' Net REPL.

use super::{deserialize_message, serialize_message, Message, NetError, PROTOCOL_VERSION};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

/// Configuration for the Calvin network server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// The address to bind to.
    pub bind_address: String,
    /// The port to listen on.
    pub port: u16,
    /// The server name (for handshake).
    pub name: String,
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_address: "0.0.0.0".to_string(),
            port: 8472,
            name: "calvin-server".to_string(),
            max_connections: 64,
        }
    }
}

/// A Calvin network server that evaluates expressions remotely.
pub struct Server {
    config: ServerConfig,
    compiler: Arc<Mutex<crate::compiler::Compiler>>,
}

impl Server {
    /// Create a new server with the given configuration.
    pub fn new(config: ServerConfig) -> Self {
        Server {
            config,
            compiler: Arc::new(Mutex::new(crate::compiler::Compiler::new())),
        }
    }

    /// Start the server and listen for connections.
    pub fn start(&self) -> Result<(), NetError> {
        let addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let listener = TcpListener::bind(&addr)?;
        tracing::info!("Calvin server listening on {}", addr);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let compiler = Arc::clone(&self.compiler);
                    let server_name = self.config.name.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, compiler, &server_name) {
                            tracing::error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Accept error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Get the server address string.
    pub fn address(&self) -> String {
        format!("{}:{}", self.config.bind_address, self.config.port)
    }
}

/// Handle a single client connection.
fn handle_connection(
    mut stream: TcpStream,
    compiler: Arc<Mutex<crate::compiler::Compiler>>,
    server_name: &str,
) -> Result<(), NetError> {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    tracing::info!("New connection from {}", peer);

    let mut buf = vec![0u8; 65536];

    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            tracing::info!("Connection closed by {}", peer);
            break;
        }

        let (msg, _consumed) = deserialize_message(&buf[..n])?;

        let response = match msg {
            Message::Handshake {
                version,
                client_name,
            } => {
                tracing::info!(
                    "Handshake from {} (version {}, client: {})",
                    peer,
                    version,
                    client_name
                );
                Message::HandshakeAck {
                    version: PROTOCOL_VERSION,
                    server_name: server_name.to_string(),
                }
            }

            Message::EvalRequest { id, expression } => {
                let result = compiler
                    .lock()
                    .map_err(|e| NetError::Server(e.to_string()))?
                    .eval_str(&expression);
                Message::EvalResponse {
                    id,
                    result: result
                        .map(|v| format!("{}", v))
                        .map_err(|e| format!("{}", e)),
                }
            }

            Message::DefineRequest {
                id,
                name,
                expression,
            } => {
                let result = compiler
                    .lock()
                    .map_err(|e| NetError::Server(e.to_string()))?
                    .define(&name, &expression);
                Message::DefineResponse {
                    id,
                    result: result.map_err(|e| format!("{}", e)),
                }
            }

            Message::ListBindingsRequest { id } => {
                let bindings = compiler
                    .lock()
                    .map_err(|e| NetError::Server(e.to_string()))?
                    .bound_names()
                    .into_iter()
                    .map(|name| (name.clone(), "".to_string()))
                    .collect();
                Message::ListBindingsResponse { id, bindings }
            }

            Message::Ping { id } => Message::Pong { id },

            Message::Shutdown => {
                tracing::info!("Shutdown requested by {}", peer);
                break;
            }

            _ => {
                tracing::warn!("Unexpected message from {}", peer);
                continue;
            }
        };

        let response_bytes = serialize_message(&response)?;
        stream.write_all(&response_bytes)?;
        stream.flush()?;
    }

    Ok(())
}
