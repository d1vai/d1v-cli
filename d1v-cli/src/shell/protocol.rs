use serde::{Deserialize, Serialize};

pub const VERSION: u8 = 1;
pub const SUBPROTOCOL: &str = "d1v-terminal.v1";
pub const INPUT_CHANNEL: u8 = 0x00;
pub const OUTPUT_CHANNEL: u8 = 0x01;
pub const STDERR_CHANNEL: u8 = 0x02;
pub const MAX_BINARY_PAYLOAD_BYTES: usize = 64 * 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("terminal binary frame is empty")]
    EmptyBinaryFrame,
    #[error("unsupported terminal binary channel {0}")]
    UnsupportedBinaryChannel(u8),
    #[error("terminal binary payload exceeds {MAX_BINARY_PAYLOAD_BYTES} bytes")]
    BinaryPayloadTooLarge,
    #[error("invalid terminal control frame: {0}")]
    InvalidControlFrame(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerBinaryChannel {
    Output,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientControl {
    Open {
        version: u8,
        cols: u16,
        rows: u16,
        term: &'static str,
    },
    Resize {
        cols: u16,
        rows: u16,
    },
    Signal {
        signal: Signal,
    },
    Ping {
        timestamp: i64,
    },
    Detach,
}

impl ClientControl {
    #[must_use]
    pub fn open(cols: u16, rows: u16) -> Self {
        Self::Open {
            version: VERSION,
            cols,
            rows,
            term: "xterm-256color",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Signal {
    #[serde(rename = "SIGINT")]
    Interrupt,
    #[serde(rename = "SIGTERM")]
    Terminate,
    #[serde(rename = "SIGHUP")]
    Hangup,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerControl {
    Ready {
        session_id: String,
        cwd: String,
    },
    Pong {
        timestamp: i64,
    },
    Cwd {
        path: String,
    },
    Exit {
        code: Option<i32>,
        signal: Option<String>,
    },
    Error {
        code: String,
        retryable: bool,
    },
}

pub fn encode_control(control: &ClientControl) -> Result<String, ProtocolError> {
    serde_json::to_string(control)
        .map_err(|error| ProtocolError::InvalidControlFrame(error.to_string()))
}

pub fn decode_control(frame: &str) -> Result<ServerControl, ProtocolError> {
    serde_json::from_str(frame)
        .map_err(|error| ProtocolError::InvalidControlFrame(error.to_string()))
}

pub fn encode_input(payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if payload.len() > MAX_BINARY_PAYLOAD_BYTES {
        return Err(ProtocolError::BinaryPayloadTooLarge);
    }
    let mut frame = Vec::with_capacity(payload.len() + 1);
    frame.push(INPUT_CHANNEL);
    frame.extend_from_slice(payload);
    Ok(frame)
}

pub fn decode_output(frame: &[u8]) -> Result<&[u8], ProtocolError> {
    let (channel, payload) = decode_server_binary(frame)?;
    if channel != ServerBinaryChannel::Output {
        return Err(ProtocolError::UnsupportedBinaryChannel(STDERR_CHANNEL));
    }
    Ok(payload)
}

pub fn decode_server_binary(frame: &[u8]) -> Result<(ServerBinaryChannel, &[u8]), ProtocolError> {
    let Some((&channel, payload)) = frame.split_first() else {
        return Err(ProtocolError::EmptyBinaryFrame);
    };
    if payload.len() > MAX_BINARY_PAYLOAD_BYTES {
        return Err(ProtocolError::BinaryPayloadTooLarge);
    }
    match channel {
        OUTPUT_CHANNEL => Ok((ServerBinaryChannel::Output, payload)),
        STDERR_CHANNEL => Ok((ServerBinaryChannel::Stderr, payload)),
        _ => Err(ProtocolError::UnsupportedBinaryChannel(channel)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_open_and_binary_input() {
        assert_eq!(
            encode_control(&ClientControl::open(120, 40)).unwrap(),
            r#"{"type":"open","version":1,"cols":120,"rows":40,"term":"xterm-256color"}"#
        );
        assert_eq!(encode_input(b"ls\t").unwrap(), b"\x00ls\t");
    }

    #[test]
    fn decodes_output_and_server_controls() {
        assert_eq!(decode_output(b"\x01hello").unwrap(), b"hello");
        assert_eq!(
            decode_server_binary(b"\x02problem").unwrap(),
            (ServerBinaryChannel::Stderr, b"problem".as_slice())
        );
        assert_eq!(
            decode_control(r#"{"type":"exit","code":7,"signal":null}"#).unwrap(),
            ServerControl::Exit {
                code: Some(7),
                signal: None,
            }
        );
    }

    #[test]
    fn rejects_invalid_binary_frames() {
        assert_eq!(
            decode_output(b"").unwrap_err(),
            ProtocolError::EmptyBinaryFrame
        );
        assert_eq!(
            decode_output(b"\x00input").unwrap_err(),
            ProtocolError::UnsupportedBinaryChannel(INPUT_CHANNEL)
        );
        assert_eq!(
            encode_input(&vec![0; MAX_BINARY_PAYLOAD_BYTES + 1]).unwrap_err(),
            ProtocolError::BinaryPayloadTooLarge
        );
    }

    #[test]
    fn rejects_unknown_control_type() {
        assert!(decode_control(r#"{"type":"unknown"}"#).is_err());
    }
}
