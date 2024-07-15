use std::{io::{Read, Write}, os::unix::net::UnixStream};

use serde::{Deserialize, Serialize};
use dusa_collection_utils::errors::{ErrorArray, ErrorArrayItem, UnifiedResult};

use crate::{DusaError, MessageType};

#[derive(Serialize, Deserialize, Debug)]
pub struct GeneralMessage {
    pub version: String,
    pub msg_type: MessageType,
    pub payload: serde_json::Value,
    pub error: Option<DusaError>,
}

/// Encodes a message with a length prefix and sends it over the stream.
pub fn send_message<T: Serialize>(stream: &mut UnixStream, message: &T, mut errors: ErrorArray) -> UnifiedResult<()> {
    let message_bytes = match serde_json::to_vec(message) {
        Ok(d) => d,
        Err(e) => {
            errors.push(ErrorArrayItem::from(e));
            return UnifiedResult::new(Err(errors))
        },
    };
    let length = message_bytes.len() as u32;
    let length_bytes = length.to_be_bytes(); // Convert length to big-endian bytes

    // Send length prefix followed by the message
    if let Err(err) = stream.write_all(&length_bytes) {
        errors.push(ErrorArrayItem::from(err));
        return UnifiedResult::new(Err(errors))
    }

    if let Err(err) = stream.write_all(&message_bytes) {
        errors.push(ErrorArrayItem::from(err));
        return UnifiedResult::new(Err(errors))
    }

    return UnifiedResult::new(Ok(()))
}

/// Reads a length-prefixed message from the stream and decodes it.
pub fn receive_message(stream: &mut UnixStream, mut errors: ErrorArray) -> UnifiedResult<GeneralMessage> {
    let mut length_bytes = [0u8; 4];

    if let Err(err) = stream.read_exact(&mut length_bytes) {
        errors.push(ErrorArrayItem::from(err));
        return UnifiedResult::new(Err(errors))
    }; // get the length

    let length = u32::from_be_bytes(length_bytes) as usize;

    let mut message_bytes = vec![0u8; length];

    if let Err(err) = stream.read_exact(&mut message_bytes) {
        errors.push(ErrorArrayItem::from(err));
        return UnifiedResult::new(Err(errors))
    } // Read the message

    let message = match serde_json::from_slice(&message_bytes) {
        Ok(d) => d,
        Err(e) => {
            errors.push(ErrorArrayItem::from(e));
            return UnifiedResult::new(Err(errors))
        },
    };

    UnifiedResult::new(Ok(message))
}