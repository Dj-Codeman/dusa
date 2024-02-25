#[path = "../shared/shared.rs"]
mod shared;
use shared::get_id;
use libc::geteuid;
use nix::unistd::{chown, Gid, Uid};
use pretty::*;
use recs::errors::{RecsError, RecsErrorType, RecsRecivedErrors};
use std::path::{Path, PathBuf};
use std::fs::canonicalize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::exit;
use system::{create_hash, is_path, truncate};
// use users::{Groups, Users, UsersCache};

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();

    // Define mode based on arguments given
    let (command, arg_1, arg_2, arg_3) = parse_arguments(&args);

    // Define mode based on arguments given
    enum ProgramMode {
        StoreFile(String, String, String),
        RetrieveFile(String, String),
        EncryptText(String),
        DecryptText(String),
        RemoveFile(String, String),

        Help,
        Invalid,
    }

    // Parse command given
    let mode: ProgramMode = match command.as_ref() {
        Some(cmd) => match cmd.as_str() {
            "encrypt-file" => match (arg_1, arg_2, arg_3) {
                (Some(owner), Some(name), Some(path)) => ProgramMode::StoreFile(owner, name, path),
                _ => ProgramMode::Invalid,
            },
            "decrypt-file" => match (arg_1, arg_2) {
                (Some(owner), Some(name)) => ProgramMode::RetrieveFile(owner, name),
                _ => ProgramMode::Invalid,
            },
            "encrypt-text" => match arg_1 {
                Some(data) => ProgramMode::EncryptText(data),
                None => ProgramMode::Invalid,
            },
            "decrypt-text" => match arg_1 {
                // data key chunk
                Some(data) => ProgramMode::DecryptText(data),
                _ => ProgramMode::Invalid,
            },
            "remove-file" => match (arg_1, arg_2) {
                (Some(owner), Some(name)) => ProgramMode::RemoveFile(owner, name),
                _ => ProgramMode::Invalid,
            },
            "list-file" => match (arg_1, arg_2) {
                (Some(_), Some(_)) => todo!(),
                _ => ProgramMode::Invalid,
            },
            "status" => ProgramMode::Invalid,
            _ => ProgramMode::Help,
        },
        None => ProgramMode::Invalid,
    };

    // Communicating with server after parsing
    match mode {
        ProgramMode::StoreFile(owner, name, path) => {
            // ensuring path exists
            let safe_path: String = match is_path(&path) {
                true => path,
                false => panic!("Path specified isn't valid"),
            };

            let absolute_path = match canonicalize(&safe_path.to_owned()) {
                Ok(d) => d,
                Err(e) => panic!("{}", e.to_string()),
            };

            // Changing owner ship of the file
            let (uid, gid) = get_id();
            set_file_ownership(&absolute_path, uid, gid);

            // Pusing commands to the array
            let mut command_data: Vec<String> = vec![];
            command_data.push(String::from("insert"));
            command_data.push(owner);
            command_data.push(name);
            command_data.push(absolute_path.into_os_string().into_string().unwrap());

            // Creating the message
            let message: String = create_message(command_data);

            // Sending the message
            match send_command(message) {
                Ok(d) => pass(&d),
                Err(e) => recs::errors::RecsRecivedErrors::display(e, false),
            }
        }

        ProgramMode::RetrieveFile(owner, name) => {
            let mut command_data: Vec<String> = vec![];
            command_data.push(String::from("retrieve"));
            command_data.push(owner);
            command_data.push(name);

            let message: String = create_message(command_data);

            match send_command(message) {
                Ok(d) => pass(&d),
                Err(e) => recs::errors::RecsRecivedErrors::display(e, false),
            }
        }

        ProgramMode::EncryptText(data) => {
            let mut command_data: Vec<String> = vec![];
            command_data.push(String::from("encrypt"));
            command_data.push(data);

            let message: String = create_message(command_data);
            notice(&message);

            match send_command(message) {
                Ok(d) => pass(&d),
                Err(e) => recs::errors::RecsRecivedErrors::display(e, false),
            }
        }

        ProgramMode::DecryptText(data) => {
            let mut command_data: Vec<String> = vec![];
            command_data.push(String::from("decrypt"));
            command_data.push(data);

            let message: String = create_message(command_data);

            match send_command(message) {
                Ok(d) => pass(&d),
                Err(e) => recs::errors::RecsRecivedErrors::display(e, false),
            }
        }

        ProgramMode::RemoveFile(owner, name) => {
            let mut command_data: Vec<String> = vec![];
            command_data.push(String::from("remove"));
            command_data.push(owner);
            command_data.push(name);

            let message: String = create_message(command_data);

            match send_command(message) {
                Ok(d) => pass(&d),
                Err(e) => recs::errors::RecsRecivedErrors::display(e, false),
            }
        }

        ProgramMode::Help => {
            help(args);
            exit(0);
        }

        ProgramMode::Invalid => {
            warn("Error: Parsing arguments failed.");
            help(args);
            exit(1);
        }
    }
}

fn parse_arguments(
    args: &[String],
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let command = args.get(1).cloned();
    let owner = args.get(2).cloned();
    let name = args.get(3).cloned();
    let path = args.get(4).cloned();

    (command, owner, name, path)
}

fn set_file_ownership(path: &PathBuf, uid: Uid, gid: Gid) {
    chown(path, Some(uid), Some(gid)).expect("Failed to set file ownership");
}

fn create_message(mut data: Vec<String>) -> String {
    // for certain functions the clients uid has to be sent too
    let current_uid: u32 = unsafe { geteuid() }; 
    data.push(format!("{}", current_uid));

    let command_string: String = data.join("-");
    let hexed_command: String = hex::encode(command_string);
    let hexed_hash: String = hex::encode(truncate(&create_hash(hexed_command.clone())[7..], 50));

    let mut secure_command_array: Vec<String> = vec![];

    secure_command_array.push(hexed_command);
    secure_command_array.push(hexed_hash);
    
    let secure_command: String = secure_command_array.join("Z");
    secure_command
}

fn send_command(command: String) -> Result<String, RecsRecivedErrors> {
    let socket_path: &Path = Path::new("/var/run/dusa/dusa.sock");

    // Connect to the Unix domain socket
    let mut stream = match UnixStream::connect(socket_path) {
        Ok(stream) => stream,
        Err(e) => {
            return Err(RecsRecivedErrors::RecsError(RecsError::new_details(
                RecsErrorType::Error,
                &format!("Socket connection error: {}", e),
            )))
        }
    };

    // Write the command to the server
    match stream.write(command.as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            return Err(RecsRecivedErrors::RecsError(RecsError::new_details(
                RecsErrorType::Error,
                &format!("Error writing to socket: {}", e),
            )))
        }
    };

    // Flush the stream to ensure all data is sent
    match stream.flush() {
        Ok(_) => (),
        Err(e) => {
            warn("Data was fucked");
            return Err(RecsRecivedErrors::RecsError(RecsError::new_details(
                RecsErrorType::Error,
                &format!("Error flushing socket: {}", e),
            )));
        }
    };

    // Read the response from the server
    let mut buffer = vec![0; 2048];
    match stream.read_to_end(&mut buffer) {
        Ok(_) => {
            // Convert the received data into a string
            let response = String::from_utf8_lossy(&buffer).to_string();
            Ok(response)
        }
        Err(e) => {
            warn("Data was fucked");
            Err(RecsRecivedErrors::RecsError(RecsError::new_details(
                RecsErrorType::Error,
                &format!("Error reading from socket: {}", e),
            )))
        }
    }
}

fn help(args: Vec<String>) {
    output(
        "YELLOW",
        &format!(
            "Usage: {} <command> <owner> <name> [file_path], {} <text> [data]",
            args[0], args[0]
        ),
    );
    output(
        "GREEN",
        "Commands: encrypt-file | decrypt-file | encrypt-text | decrypt-text | remove-file | list-file | status ",
    );
}
