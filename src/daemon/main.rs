#[path = "../shared/shared.rs"]
mod shared;
mod errors;
use shared::{convert_to_string, get_id};
use nix::unistd::{chown, setgid, setuid, Gid, Uid};
use pretty::{notice, output};
use recs::errors::RecsRecivedErrors;
use recs::{decrypt_raw, encrypt_raw, initialize, insert, ping, remove, retrive, update_map};
use std::fs::create_dir;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::{fs, thread};
use system::{del_file, is_path};

fn main() {
    // Make sure we are running as the dusa user
    let (uid, gid) = get_id();

    let _ = setuid(uid.into());
    let _ = setgid(gid.into());


    // Initializing the recs lib properly
    recs::set_debug(true);
    recs::set_prog("dusa");

    // Defining where the socket file is 
    let _ = match is_path("/var/run/dusa") {
        true => (), // nothing no folder is needed
        false => create_dir("/var/run/dusa").unwrap(),
    };

    let socket_path: &str = "/var/run/dusa/dusa.sock";

    // Setting up the new socket file
    let _ = del_file(socket_path);
    let listener: UnixListener = match UnixListener::bind(socket_path) {
        Ok(d) => d,
        Err(e) => panic!("Socket binding error: {}", e),
    };

    // Changing the permissions the socket
    let socket_metadata = fs::metadata(socket_path).unwrap();
    let mut permissions = socket_metadata.permissions();
    permissions.set_mode(0o770);  // Set desired permissions
    fs::set_permissions(socket_path, permissions).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Spawn a new thread or use async/await to handle each incoming connection
                thread::spawn(|| handle_client(stream));
            },
            Err(e) => eprintln!("Error accepting connection: {}", e),
        }
    }
    
}

fn handle_client(mut stream: UnixStream) {
    // Create a buffer to hold incoming data
    let mut buffer = vec![0; 8960];

    // Read data from the client in a loop
    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    // Connection closed
                    break;
                }

                // Convert the received data into a string
                let command_str = convert_to_string(&buffer[..size]);
                notice(&command_str);
                
                if size == buffer.len() {
                    buffer.resize(buffer.len() * 2, 0);
                }

                let response = hex::encode(process_command(command_str));

                // Write the response back to the client
                if let Err(e) = stream.write_all(response.as_bytes()) {
                    eprintln!("Error writing to client: {}", e);
                    break;
                } else {
                    notice(&response);
                }
            }
            Err(e) => {
                eprintln!("Error reading from client: {}", e);
                break;
            }
        }
    }

    // Shutdown the connection gracefully
    let _ = stream.shutdown(Shutdown::Both);
}

fn process_command(command_str: String) -> String {
    // Ensure data is initialized before processing command
    match initialize() {
        Ok(_) => (),
        Err(e) => 
            RecsRecivedErrors::display(e, true),
    }

    // let parts: Vec<&str> = command_str.split_whitespace().collect();
    let parts: Vec<&str> = command_str.split('-').collect();

    match parts.get(0) {
        Some(&"insert") => {
            let filename = parts.get(1).unwrap_or(&"").to_string();
            let owner = parts.get(2).unwrap_or(&"").to_string();
            let name = parts.get(3).unwrap_or(&"").to_string();
            // Taking ownership of the file 
            match insert(filename, owner, name) {
                Ok(_) => "Inserted Successfully".to_string(),
                Err(e) => {
                RecsRecivedErrors::display(e, false);
                panic!(); },
            }
        }
        Some(&"retrieve") => {
            let owner = parts.get(1).unwrap_or(&"").to_string();
            let name = parts.get(2).unwrap_or(&"").to_string();
            match retrive(owner, name) {
                Ok(_) => "Retrieved Successfully".to_string(),
                Err(e) => panic!("{:?}", RecsRecivedErrors::display(e, false)),
            }
        }
        Some(&"remove") => {
            let owner = parts.get(1).unwrap_or(&"").to_string();
            let name = parts.get(2).unwrap_or(&"").to_string();
            match remove(owner, name) {
                Ok(_) => "Removed Successfully".to_string(),
                Err(e) => panic!("{:?}", RecsRecivedErrors::display(e, false)),
            }
        }
        Some(&"ping") => {
            let owner = parts.get(1).unwrap_or(&"").to_string();
            let name = parts.get(2).unwrap_or(&"").to_string();
            ping(owner, name).to_string()
        }
        Some(&"encrypt") => {
            let data = parts.get(1).unwrap_or(&"").to_string();
            match encrypt_raw(data) {
                Ok((key, cipher, chunks)) => {
                    format!("Key: {}, Cipher: {}, Chunks: {}", key, cipher, chunks)
                }
                Err(e) => panic!("{:?}", RecsRecivedErrors::display(e, false)),
            }
        }
        Some(&"decrypt") => {
            let recs_data = parts.get(1).unwrap_or(&"").to_string();
            let recs_key = parts.get(2).unwrap_or(&"").to_string();
            let recs_chunks = parts.get(3).unwrap_or(&"0").parse::<usize>().unwrap_or(0);
            match decrypt_raw(recs_data, recs_key, recs_chunks) {
                Ok(data) => format!("Decrypted Data: {:?}", data),
                Err(e) => panic!("{:?}", RecsRecivedErrors::display(e, false)),
            }
        }
        Some(&"update_map") => {
            let map_num = parts.get(1).unwrap_or(&"0").parse::<u32>().unwrap_or(0);
            if update_map(map_num) {
                "Map Updated Successfully".to_string()
            } else {
                "Failed to Update Map".to_string()
            }
        }
        _ => "Invalid Command".to_string(),
    }
}
