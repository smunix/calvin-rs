//! Networking and IPC for the Calvin language.
//!
//! This module provides TCP and Unix domain socket networking for
//! remote expression evaluation (Net REPL), similar to the hobbes
//! networking layer.

pub mod client;
pub mod server;

use thiserror::Error;

/// Errors that can occur during networking operations.
#[derive(Debug, Error)]
pub enum NetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection refused: {0}")]
    ConnectionRefused(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Timeout")]
    Timeout,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Server error: {0}")]
    Server(String),
}

/// The protocol version for Calvin network communication.
pub const PROTOCOL_VERSION: u32 = 1;

/// Message types for the Calvin network protocol.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    /// A request to evaluate an expression.
    EvalRequest {
        id: u64,
        expression: String,
    },
    /// The result of evaluating an expression.
    EvalResponse {
        id: u64,
        result: Result<String, String>,
    },
    /// A request to define a variable.
    DefineRequest {
        id: u64,
        name: String,
        expression: String,
    },
    /// The result of a define operation.
    DefineResponse {
        id: u64,
        result: Result<(), String>,
    },
    /// A request to list all bindings.
    ListBindingsRequest {
        id: u64,
    },
    /// The list of all bindings.
    ListBindingsResponse {
        id: u64,
        bindings: Vec<(String, String)>,
    },
    /// A request to get the type of an expression.
    TypeRequest {
        id: u64,
        expression: String,
    },
    /// The type of an expression.
    TypeResponse {
        id: u64,
        result: Result<String, String>,
    },
    /// A ping message for keepalive.
    Ping {
        id: u64,
    },
    /// A pong response.
    Pong {
        id: u64,
    },
    /// A shutdown request.
    Shutdown,
    /// A handshake message.
    Handshake {
        version: u32,
        client_name: String,
    },
    /// A handshake acknowledgment.
    HandshakeAck {
        version: u32,
        server_name: String,
    },
}

/// Serialize a message to bytes (length-prefixed JSON).
pub fn serialize_message(msg: &Message) -> Result<Vec<u8>, NetError> {
    let json = serde_json::to_string(msg).map_err(|e| NetError::Serialization(e.to_string()))?;
    let len = json.len() as u32;
    let mut buf = Vec::with_capacity(4 + json.len());
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(json.as_bytes());
    Ok(buf)
}

/// Deserialize a message from bytes (length-prefixed JSON).
pub fn deserialize_message(data: &[u8]) -> Result<(Message, usize), NetError> {
    if data.len() < 4 {
        return Err(NetError::Protocol("Message too short".to_string()));
    }
    let len = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    if data.len() < 4 + len {
        return Err(NetError::Protocol("Incomplete message".to_string()));
    }
    let json = std::str::from_utf8(&data[4..4 + len])
        .map_err(|e| NetError::Protocol(e.to_string()))?;
    let msg: Message =
        serde_json::from_str(json).map_err(|e| NetError::Serialization(e.to_string()))?;
    Ok((msg, 4 + len))
}

/// Parse a host:port string.
pub fn parse_address(addr: &str) -> Result<(String, u16), NetError> {
    let parts: Vec<&str> = addr.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(NetError::Protocol(format!("Invalid address: {}", addr)));
    }
    let port = parts[0]
        .parse::<u16>()
        .map_err(|_| NetError::Protocol(format!("Invalid port: {}", parts[0])))?;
    let host = parts[1].to_string();
    Ok((host, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_roundtrip() {
        let msg = Message::EvalRequest {
            id: 1,
            expression: "1 + 2".to_string(),
        };
        let bytes = serialize_message(&msg).unwrap();
        let (decoded, consumed) = deserialize_message(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        if let Message::EvalRequest { id, expression } = decoded {
            assert_eq!(id, 1);
            assert_eq!(expression, "1 + 2");
        } else {
            panic!("Wrong message type");
        }
    }

    #[test]
    fn test_parse_address() {
        let (host, port) = parse_address("localhost:8080").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_address_invalid() {
        assert!(parse_address("invalid").is_err());
    }
}
