pub mod prefix;

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use nix::unistd::{chown, Gid, Uid};
use pretty::halt;
use serde::{Deserialize, Serialize};
use system::{
    errors::{
        ErrorArray, ErrorArrayItem, Errors as SE, OkWarning, UnifiedResult as uf, WarningArray,
        WarningArrayItem, Warnings,
    },
    functions::del_file,
    types::PathType,
};
use users::{Groups, Users, UsersCache};

/// Current version of the protocol, derived from the package version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Time to live in seconds for file that are decrypted.
pub const TTL: u64 = 5;

/// Getting the current uid
pub fn get_id() -> (Uid, Gid) {
    let user_cache: UsersCache = UsersCache::new();
    let dusa_uid = user_cache.get_user_by_name("dusa").unwrap().uid();
    let dusa_gid = user_cache.get_group_by_name("dusa").unwrap().gid();

    (Uid::from_raw(dusa_uid), Gid::from_raw(dusa_gid))
}

/// Struct representing a write request.
#[derive(Serialize, Deserialize, Debug)]
pub struct RequestRecsWrite {
    pub path: PathType,
    pub owner: String,
    pub name: String,
    pub uid: u32,
}

/// Struct representing a plain text request.
#[derive(Serialize, Deserialize, Debug)]
pub struct RequestRecsPlainText {
    pub command: Commands,
    pub data: String,
    pub uid: u32,
}

/// Struct representing a simple request.
#[derive(Serialize, Deserialize, Debug)]
pub struct RequestRecsSimple {
    pub command: Commands,
    pub owner: String,
    pub name: String,
    pub uid: u32,
}

/// Struct representing a response.
#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseData {
    pub status: String,
    pub detail: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DecryptResponseData{
    pub temp_p: PathType,
    pub orig_p: PathType,
    pub ttl: Duration,
}

/// Enum representing different request payloads.
#[derive(Serialize, Deserialize, Debug)]
pub enum RequestPayload {
    Write(RequestRecsWrite),
    PlainText(RequestRecsPlainText),
    Simple(RequestRecsSimple),
}

/// enums for commands 
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Commands {
    EncryptRawText,
    DecryptRawText,
    DecryptFile,
    RemoveFile,
    PingFile,
}

/// Generic message struct used for communication.
#[derive(Serialize, Deserialize, Debug)]
pub struct Message<T> {
    pub version: String,
    pub msg_type: MessageType,
    pub payload: T,
    pub error: Option<DusaError>,
}

/// Enum representing different message types.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum MessageType {
    Request,
    Response,
    ErrorResponse,
    Simple,
    Acknowledge,
    Test,
    // Add more custom message types as needed
}

/// Enum representing different error codes.
#[derive(Serialize, Deserialize, Debug)]
pub enum ErrorCode {
    UnknownMessageType,
    InvalidPayload,
    InvalidVersion,
    InternalError,
    InvalidPermissions,
    // Add more standardized error codes as needed
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::UnknownMessageType => write!(f, "Unknown message type"),
            ErrorCode::InvalidPayload => write!(f, "Invalid payload"),
            ErrorCode::InternalError => write!(f, "Internal error"),
            ErrorCode::InvalidVersion => write!(f, "We aren't speaking the same language"),
            ErrorCode::InvalidPermissions => write!(f, "You have no authority here"),
            // Add more standardized error codes as needed
        }
    }
}

impl std::fmt::Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::EncryptRawText => write!(f, "et"),
            Commands::DecryptRawText => write!(f, "dt"),
            Commands::DecryptFile => write!(f, "df"),
            Commands::RemoveFile => write!(f, "rf"),
            Commands::PingFile => write!(f, "pf"),
        }
    }
}

/// Struct representing an error message.
#[derive(Serialize, Deserialize, Debug)]
pub struct DusaError {
    pub code: ErrorCode,
    pub message: String,
}

pub fn check_version(incoming_version: &str) -> bool {
    // Split the version strings into major, minor, and patch parts
    let parse_version = |v: &str| -> Option<(u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        let _patch: u32 = parts[2].parse::<u32>().ok()?;
        Some((major, minor))
    };

    if let (Some((inc_major, inc_minor)), Some((ver_major, ver_minor))) =
        (parse_version(incoming_version), parse_version(VERSION))
    {
        inc_major == ver_major && inc_minor == ver_minor
    } else {
        false
    }
}


/// Returns the path to the socket.
///
/// # Arguments
/// * `int` - A boolean indicating if initialization is needed.
/// * `errors` - An array of errors to be populated if any occur.
/// * `warnings` - An array of warnings to be populated if any occur.
///
/// # Returns
/// A unified result containing the path type or errors/warnings.
#[allow(nonstandard_style)]
pub fn SOCKET_PATH(
    int: bool,
    mut errors: ErrorArray,
    mut warnings: WarningArray,
) -> uf<OkWarning<PathType>> {
    let socket_file = PathType::Content(String::from("/var/run/dusa/dusa.sock"));
    // let socket_file = PathType::Content(String::from("/home/dwhitfield/Developer/RUST/Dev/server/s.socket"));
    let _socket_dir = match socket_file.ancestors().next() {
        Some(d) => PathType::PathBuf(d.to_path_buf()),
        None => {
            errors.push(ErrorArrayItem::new(
                SE::InvalidFile,
                "Socket file not found".to_string(),
            ));
            return uf::new(Err(errors));
        }
    };

    if int {
        // Create the dir and the sock file
        if socket_file.exists() {
            match del_file(socket_file.clone(), errors.clone(), warnings.clone()).uf_unwrap() {
                Ok(_) => {
                    return uf::new(Ok(OkWarning {
                        data: socket_file,
                        warning: warnings,
                    }));
                }
                Err(_) => {
                    warnings.push(WarningArrayItem::new(Warnings::OutdatedVersion));
                }
            }
        }
    }

    uf::new(Ok(OkWarning {
        data: socket_file,
        warning: warnings,
    }))
}

pub fn set_file_ownership(path: &PathBuf, uid: Uid, gid: Gid, mut errors: ErrorArray) -> uf<()> {
    match chown(path, Some(uid), Some(gid)) {
        Ok(_) => uf::new(Ok(())),
        Err(_) => {
            errors.push(ErrorArrayItem::new(
                system::errors::Errors::Unauthorized,
                String::from("chown failed"),
            ));
            uf::new(Err(errors))
        }
    }
}

pub fn set_socket_permission(socket_path: PathType) {
    // Changing the permissions the socket
    let socket_metadata = match fs::metadata(socket_path.clone()) {
        Ok(d) => d,
        Err(e) => {
            halt(&format!(
                "Couldn't read meta data of the socket: {}",
                &e.to_string()
            ));
            unreachable!()
        }
    };
    let mut permissions = socket_metadata.permissions();
    permissions.set_mode(0o660); // Set desired permissions

    match fs::set_permissions(socket_path.clone(), permissions) {
        Ok(()) => (),
        Err(e) => halt(&format!(
            "We own the socket but we can't change its permissions, all i know is '{}'",
            &e.to_string()
        )),
    };
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Request => write!(f, "Request"),
            MessageType::Response => write!(f, "Response"),
            MessageType::ErrorResponse => write!(f, "Error"),
            MessageType::Simple => write!(f, "Simple Message"),
            MessageType::Acknowledge => write!(f, "Understood"),
            MessageType::Test => write!(f, "Test message"),
            // Add more custom message types as needed
        }
    }
}

