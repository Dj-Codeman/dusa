mod errors;
use common::shared::{convert_to_string, get_id, no_kay_val, okay_val};
use common::warn::{Errors, Warnings};
use common::SOCKET_PATH;
use nix::unistd::{setgid, setuid};
use pretty::{halt, notice, output, warn};
use recs::errors::RecsRecivedErrors;
use recs::{decrypt_raw, encrypt_raw, initialize, insert, ping, remove, retrive, update_map};
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::{fs, thread};
use system::{create_hash, truncate, ClonePath, PathType};

fn main() {
    // Make sure we are running as the dusa user
    let (uid, gid) = get_id();

    match (setuid(uid.into()), setgid(gid.into())) {
        (Ok(_), Ok(_)) => (),
        _ => halt("We aren't running as the correct user, peacing out .."),
    };

    // Initializing the recs lib properly
    recs::set_debug(true);
    recs::set_prog("dusa");

    // Setting up the new socket file
    let listener: UnixListener = match UnixListener::bind(
        SOCKET_PATH(true, Warnings::new_container(), Errors::new_container())
            .resolve()
            .clone_path(),
    ) {
        Ok(d) => d,
        Err(e) => {
            halt(&format!(
                "We couldn't create the socket because this happened: {}",
                &e.to_string()
            ));
            panic!()
        }
    };

    // Changing the permissions the socket
    let socket_metadata = match fs::metadata(
        SOCKET_PATH(false, Warnings::new_container(), Errors::new_container())
            .resolve()
            .clone_path(),
    ) {
        Ok(d) => d,
        Err(e) => {
            halt(&format!(
                "Couldn't read meta data of the socket: {}",
                &e.to_string()
            ));
            panic!()
        }
    };
    let mut permissions = socket_metadata.permissions();
    permissions.set_mode(0o660); // Set desired permissions

    match fs::set_permissions(
        SOCKET_PATH(false, Warnings::new_container(), Errors::new_container()).resolve(),
        permissions,
    ) {
        Ok(()) => (),
        Err(e) => halt(&format!(
            "We own the socket but we can't change its permissions, all i know is '{}'",
            &e.to_string()
        )),
    };

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Spawn a new thread or use async/await to handle each incoming connection
                thread::spawn(|| handle_client(stream));
            }
            Err(e) => halt(&format!("Error accepting connection: {}", e)),
        }
    }
}

fn handle_client(mut stream: UnixStream) {
    // Create a buffer to hold incoming data
    let mut buffer = vec![0; 40960];

    // Read data from the client in a loop
    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    // Connection closed
                    break;
                }

                // Convert the received data into a string
                let command_str: String = validate_command(&buffer[..size]);
                notice("Command received, processing");

                if size == buffer.len() {
                    buffer.resize(buffer.len() * 4, 0);
                }

                let response = process_command(command_str);

                // Write the response back to the client
                match stream.write(response.as_bytes()) {
                    Ok(_) => {
                        notice("Response Sent");
                        break;
                    }
                    Err(e) => {
                        halt(&format!("Error writing to client: {}", e));
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading from client: {}", e);
                break;
            }
        }
    }

    // Shutdown the connection gracefully
    match stream.shutdown(Shutdown::Both) {
        Ok(_) => output("GREEN", "Finished, closing connection"),
        Err(e) => halt(&e.to_string()),
    }
}

fn validate_command(buffer: &[u8]) -> String {
    let unsafe_string: String = unsafe { String::from_utf8_unchecked(buffer.to_vec()) };
    let unsafe_array: Vec<&str> = unsafe_string.split("Z").collect();
    let un_validated_command: String = unsafe_array[0].to_owned();
    let un_validated_hash: String = unsafe_array[1].to_owned();
    let valid_hash: String = hex::encode(truncate(
        &create_hash(un_validated_command.clone())[7..],
        50,
    ));
    match un_validated_hash == valid_hash {
        // TODO add some permission checks on what was requested
        true => convert_to_string(un_validated_command.as_bytes()),
        false => {
            halt("YOUR COMMANDS AIN'T GOOD ROUND HERE SON");
            String::from("GIVE ME ALL THE DATA, sike ass dude")
        }
    }
}

fn process_command(command_str: String) -> String {
    // Ensure data is initialized before processing command
    match initialize() {
        Ok(_) => (),
        Err(e) => RecsRecivedErrors::display(e, true),
    }

    // let parts: Vec<&str> = command_str.split_whitespace().collect();
    let parts: Vec<&str> = command_str.split('*').collect();

    match parts.get(0) {
        Some(&"0x000") => {
            let owner: String = parts.get(1).unwrap_or(&"").to_string();
            let name: String = parts.get(2).unwrap_or(&"").to_string();
            let path: PathType = PathType::Content(parts.get(3).unwrap_or(&"").to_string());
            // Taking ownership of the file
            match insert(path, owner, name) {
                Ok(_) => okay_val(None),
                Err(e) => {
                    warn(&format!("{:?}", e));
                    no_kay_val()
                }
            }
        }
        Some(&"0x010") => {
            let owner = parts.get(1).unwrap_or(&"").to_string();
            let name = parts.get(2).unwrap_or(&"").to_string();
            let uid: u32 = match parts.get(3).unwrap_or(&"").parse::<u32>() {
                Ok(uid) => uid,
                Err(e) => panic!("{}", &e.to_string()),
            };
            match retrive(owner, name, uid) {
                Ok((file_path, file_home)) => {
                    let mut response: Vec<String> = vec![];
                    response.push(file_path.to_string());
                    response.push(file_home.to_string());
                    okay_val(Some(response))
                }

                Err(e) => {
                    warn(&format!("{:?}", e));
                    no_kay_val()
                }
            }
        }
        Some(&"0x100") => {
            let owner = parts.get(1).unwrap_or(&"").to_string();
            let name = parts.get(2).unwrap_or(&"").to_string();
            match remove(owner, name) {
                Ok(_) => okay_val(None),
                Err(e) => {
                    warn(&format!("{:?}", e));
                    no_kay_val()
                }
            }
        }
        Some(&"0x110") => {
            let owner = parts.get(1).unwrap_or(&"").to_string();
            let name = parts.get(2).unwrap_or(&"").to_string();
            match ping(owner, name).unwrap() {
                true => okay_val(None),
                false => no_kay_val(),
            }
        }
        Some(&"0x001") => {
            let data = parts.get(1).unwrap_or(&"").to_string();
            match encrypt_raw(data) {
                Ok((key, cipher, chunks)) => {
                    let mut response: Vec<String> = vec![];
                    response.push(key);
                    response.push(cipher);
                    response.push(format!("{}", chunks));
                    okay_val(Some(response))
                }
                Err(e) => {
                    warn(&format!("{:?}", e));
                    no_kay_val()
                }
            }
        }
        Some(&"0x011") => {
            let hexed_data: String = parts.get(1).unwrap_or(&"").to_string();
            let un_hexed_data: String = match &hex::decode(hexed_data) {
                Ok(d) => unsafe { String::from_utf8_unchecked(d.to_vec()) },
                Err(e) => {
                    warn(&format!("{:?}", e));
                    panic!();
                }
            };

            notice(&un_hexed_data);

            let parts_array: Vec<&str> = un_hexed_data.split('=').collect(); //= is the delimiter for the command interchange
            let recs_key = parts_array[0].to_owned();
            let recs_data = parts_array[1].to_owned();
            let recs_chunks = parts_array[2].parse::<usize>().unwrap_or(1);

            match decrypt_raw(recs_data, recs_key, recs_chunks) {
                Ok(data) => okay_val(Some(vec![String::from_utf8(data).unwrap()])),
                Err(e) => {
                    warn(&format!("{:?}", e));
                    no_kay_val()
                }
            }
        }
        Some(&"0x111") => {
            let map_num = parts.get(1).unwrap_or(&"0").parse::<u32>().unwrap_or(0);
            if update_map(map_num) {
                okay_val(None)
            } else {
                no_kay_val()
            }
        }
        _ => no_kay_val(),
    }
}
