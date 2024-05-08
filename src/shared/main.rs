use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

use libc::geteuid;
use recs::errors::{RecsError, RecsErrorType, RecsWarning, RecsWarningType};
use shared::Actions;
use system::{create_hash, del_file, truncate, ClonePath, PathType};
use warn::{Errors, OkWarning, UnifiedResult as uf, Warnings};

pub mod shared;
pub mod warn;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// THIS FUNCTION RETURNS THE PATH TO THE SOCKET  
#[allow(nonstandard_style)]
pub fn SOCKET_PATH(int: bool, mut w: Warnings, mut e: Errors) -> uf<PathType> {
    let socket_file: PathType = PathType::Content(String::from("/var/run/dusa/dusa.sock"));
    let _socket_dir: PathType = PathType::PathBuf(match socket_file.ancestors().next() {
        Some(d) => d.to_path_buf(),
        None => {
            e.0.push(RecsError::new_details(
                recs::errors::RecsErrorType::InvalidFile,
                &"Socket file not found".to_owned(),
            ));
            return uf::new(Err(e));
        }
    });

    match int {
        true => {
            // Create the dir and the sock file
            let socket_file_exists = socket_file.exists();
            match socket_file_exists {
                true => match del_file(socket_file.clone_path()) {
                    Ok(_) => {
                        let result: OkWarning<PathType> = OkWarning {
                            data: socket_file,
                            warning: w,
                        };
                        return uf::new(Ok(result));
                    }
                    Err(_) => {
                        w.0.push(RecsWarning::new(RecsWarningType::OutdatedVersion));
                        let result: OkWarning<PathType> = OkWarning {
                            data: socket_file,
                            warning: w,
                        };
                        return uf::new(Ok(result));
                    }
                },
                false => {
                    let result: OkWarning<PathType> = OkWarning {
                        data: socket_file,
                        warning: w,
                    };
                    return uf::new(Ok(result));
                }
            }
        }
        false => {
            return uf::new(Ok(OkWarning {
                data: socket_file,
                warning: w,
            }))
        }
    }
}

/// This function just a data argument as well as a command type. ONLY `ENCRYPTTEXT` and `DECRYPTTEXT`
pub fn create_simple_command(command: Actions, data: String) -> uf<String> {
    let mut errors: Errors = Errors::new_container();
    let mut command_msg: Vec<String> = Vec::new();

    match command {
        Actions::EncryptText => {
            command_msg.push(Actions::EncryptText.to_string());
            let hexed_data: String = hex::encode(data);
            command_msg.push(hexed_data);
            let message: String = create_message(command_msg);
            return uf::new(Ok(OkWarning {
                data: message,
                warning: Warnings::new_container(),
            }));
        }
        Actions::DecryptText => {
            command_msg.push(Actions::EncryptText.to_string());
            let hexed_data: String = hex::encode(data);
            command_msg.push(hexed_data);
            let message: String = create_message(command_msg);
            return uf::new(Ok(OkWarning {
                data: message,
                warning: Warnings::new_container(),
            }));
        }
        _ => {
            errors.0.push(RecsError::new(
                recs::errors::RecsErrorType::InvalidAuthRequest,
            ));
            return uf::new(Err(errors));
        }
    }
}

pub fn create_message(mut data: Vec<String>) -> String {
    // for certain functions the clients uid has to be sent too
    let current_uid: u32 = unsafe { geteuid() };
    data.push(format!("{}", current_uid));

    let command_string: String = data.join("*");
    let hexed_command: String = hex::encode(command_string);
    let hexed_hash: String = hex::encode(truncate(&create_hash(hexed_command.clone())[7..], 50));

    let mut secure_command_array: Vec<String> = vec![];

    secure_command_array.push(hexed_command);
    secure_command_array.push(hexed_hash);

    let secure_command: String = secure_command_array.join("Z");
    secure_command
}

pub fn send_command(command: String) -> Result<OkWarning<String>, RecsError> {
    // Connect to the Unix domain socket
    let mut stream = match UnixStream::connect(
        SOCKET_PATH(false, Warnings::new_container(), Errors::new_container()).resolve(),
    ) {
        Ok(stream) => stream,
        Err(e) => {
            return Err(RecsError::new_details(
                RecsErrorType::Error,
                &format!("Socket connection error: {}", e),
            ))
        }
    };

    // Write the command to the server
    match stream.write(command.as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            return Err(RecsError::new_details(
                RecsErrorType::Error,
                &format!("Error writing to socket: {}", e),
            ))
        }
    };

    // Flush the stream to ensure all data is sent
    match stream.flush() {
        Ok(_) => (),
        Err(e) => {
            return Err(RecsError::new_details(
                RecsErrorType::Error,
                &format!("Error flushing socket: {}", e),
            ));
        }
    };

    // Read the response from the server
    let mut buffer: Vec<u8> = vec![0; 89200];
    match stream.read_to_end(&mut buffer) {
        Ok(_) => {
            // Convert the received data into a string
            let response = String::from_utf8_lossy(&buffer).to_string();
            Ok(OkWarning {
                data: response,
                warning: Warnings::new_container(),
            })
        }
        Err(e) => {
            return Err(RecsError::new_details(
                RecsErrorType::Error,
                &format!("Error reading from socket: {}", e),
            ));
        }
    }
}
