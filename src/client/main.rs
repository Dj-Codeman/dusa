use users::{UsersCache, Users, Groups};
use nix::unistd::{chown, Gid, Uid};
use pretty::*;
use recs::errors::{RecsError, RecsErrorType, RecsRecivedErrors};
use std::env;
use std::fs::canonicalize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::exit;

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

	// Getting a cached list of users from the os
	let user_cache: UsersCache  = UsersCache::new();

    // Define mode based on arguments given
    enum ProgramMode {
        Writing(String, String, String, String),
        Manage(String, String, String), // 
        Text(String),
        Help,
        Invalid,
    }

    let (command, owner, name, path): (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = if args.len() > 0 {
        let command: Option<String> = args.get(1).map(|data| data.to_owned());
        let owner: Option<String> = args.get(2).map(|data| data.to_owned());
        let name: Option<String> = args.get(3).map(|data| data.to_owned());
        let path: Option<String> = args.get(4).map(|data| data.to_owned());

        (command, owner, name, path)
    } else {
        (None, None, None, None)
    };

    // Parse command given
    let mode: ProgramMode = match (command, owner, name, path) {
        (None, None, None, None) => ProgramMode::Help,
        (Some(_), None, None, None) => ProgramMode::Help,
        (Some(command), Some(text_data), None, None) => match command.as_str() {
            "text" => ProgramMode::Text(text_data),
            _ => ProgramMode::Help,
        },
        (Some(command), Some(owner), Some(name), None) => ProgramMode::Manage(command, owner, name),
        (Some(command), Some(owner), Some(name), Some(path)) => match command.as_str() {
            "insert" => ProgramMode::Writing(command, owner, name, path),
            _ => ProgramMode::Help,
        },
        (_, _, _, _) => ProgramMode::Invalid,
    };

    match mode {
        ProgramMode::Writing(command, owner, name, path) => {
            let message = format!("{} {} {} {}", command, path, owner, name);
			let dusa_uid = user_cache.get_user_by_name("dusa").unwrap();
			let dusa_gid = user_cache.get_group_by_name("dusa").unwrap();
			notice(&format!("{:?}", &dusa_gid.gid()));
			notice(&format!("{:?}", &dusa_uid.uid()));

            chown(&canonicalize(&path).unwrap(), Some(Uid::from_raw(dusa_uid.uid())), Some(Gid::from_raw(dusa_gid.gid()))).unwrap(); // What 
            match send_command(message) {
                Ok(response) => pass(&response),
                Err(e) => recs::errors::RecsRecivedErrors::display(e, true),
            }
        }

        ProgramMode::Manage(command, owner, name) => {
            let message = format!("{} {} {}", command, owner, name);
            match send_command(message) {
                Ok(response) => pass(&response),
                Err(e) => recs::errors::RecsRecivedErrors::display(e, true),
            }
        }

        ProgramMode::Text(_) => {
            warn("Error: The 'text' command is not supported. Encryption is handled server-side.");
            exit(1);
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

fn send_command(command: String) -> Result<String, RecsRecivedErrors> {
    let socket_path = "/var/run/dusa/dusa.sock";
    let mut stream = match UnixStream::connect(socket_path) {
        Ok(d) => d,
        Err(e) => return Err(RecsRecivedErrors::RecsError(RecsError::new_details(
            RecsErrorType::Error,
            &format!("socket err: {}", e.to_string()),
        ))),
    };

    match stream.write_all(command.as_bytes()) {
        Ok(_) => (),
        Err(e) => return Err(RecsRecivedErrors::RecsError(RecsError::new_details(
            RecsErrorType::Error,
            &format!("socket err: {}", e.to_string()),
        ))),
    };

    let mut buffer = Vec::new();
    match stream.read_to_end(&mut buffer){
        Ok(d) => d,
        Err(e) => return Err(RecsRecivedErrors::RecsError(RecsError::new_details(
            RecsErrorType::Error,
            &e.to_string(),
        ))),
    };

    Ok(String::from_utf8_lossy(&buffer).to_string())
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
        "Commands: insert | retrieve | remove | ping | text ",
    );
}
