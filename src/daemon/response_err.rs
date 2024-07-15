use std::os::unix::net::UnixStream;

use dusa_common::{prefix::{send_message, GeneralMessage}, DusaError, ErrorCode, MessageType, VERSION};
use dusa_collection_utils::errors::{ErrorArray, UnifiedResult};

pub fn internal_error(err: &str) -> GeneralMessage {
    let error = DusaError {
        code: ErrorCode::InternalError,
        message: err.to_string(),
    };
    return GeneralMessage {
        version: VERSION.to_owned(),
        msg_type: MessageType::ErrorResponse,
        payload: serde_json::json!({"error": "Internal Server Error"}),
        error: Some(error),
    };
}

pub fn acknowledge(stream: &mut UnixStream, errors: ErrorArray) -> UnifiedResult<()> {
    let ack = GeneralMessage {
        version: VERSION.to_owned(),
        msg_type: MessageType::Acknowledge,
        payload: serde_json::json!({}),
        error: None,
    };
    send_message(stream, &ack, errors.clone())
}
