mod cli;
use cli::build_cli;
use common::shared::{convert_to_string, get_id, no_kay_val, okay_val, Actions};
use common::warn::{Errors, OkWarning, UnifiedResult as uf, Warnings};
use common::{create_message, send_command};
use nix::unistd::{chown, Gid, Uid};
use pretty::*;
use recs::errors::{RecsError, RecsErrorType};
use std::path::PathBuf;
use std::process::exit;
use system::PathType;

type Callback = fn(clap::ArgMatches, Warnings, Errors) -> uf<String>;

enum ProgramMode {
    StoreFile(Callback),
    RetrieveFile(Callback),
    EncryptText(Callback),
    DecryptText(Callback),
    RemoveFile(Callback),
    Invalid,
}

fn main() {
    // clapping
    let cmd: clap::ArgMatches = build_cli().get_matches();

    // Get operating mode
    let (ef, df, et, dt, rf) = (
        cmd.get_flag("encrypt_file"),
        cmd.get_flag("decrypt_file"),
        cmd.get_flag("encrypt_text"),
        cmd.get_flag("decrypt_text"),
        cmd.get_flag("remove_file"),
    );

    let mode: ProgramMode = match (ef, df, et, dt, rf) {
        (true, false, false, false, false) => ProgramMode::StoreFile(encrypt_file),
        (false, true, false, false, false) => ProgramMode::RetrieveFile(decrypt_file),
        (false, false, true, false, false) => ProgramMode::EncryptText(encrypt_text),
        (false, false, false, true, false) => ProgramMode::DecryptText(decrypt_text),
        (false, false, false, false, true) => ProgramMode::RemoveFile(remove_file),
        _ => ProgramMode::Invalid,
    };

    let result: uf<String> = match mode {
        ProgramMode::StoreFile(callback) => {
            callback(cmd, Warnings::new_container(), Errors::new_container())
        }
        ProgramMode::RetrieveFile(callback) => {
            callback(cmd, Warnings::new_container(), Errors::new_container())
        }
        ProgramMode::EncryptText(callback) => {
            callback(cmd, Warnings::new_container(), Errors::new_container())
        }
        ProgramMode::DecryptText(callback) => {
            callback(cmd, Warnings::new_container(), Errors::new_container())
        }
        ProgramMode::RemoveFile(callback) => {
            callback(cmd, Warnings::new_container(), Errors::new_container())
        }
        ProgramMode::Invalid => {
            warn("Error: Parsing arguments failed.");
            exit(1)
        }
    };

    if result.clone().resolve() == okay_val(None) {
        pass("Task Finished");
    } else if result.clone().resolve() == no_kay_val() {
        halt("Errors have happened");
    } else if result.clone().resolve().is_empty() {
        dump("Result is empty")
    } else {
        notice(&format!("{}", result.resolve()))
    }
}

fn encrypt_file(cmd: clap::ArgMatches, warnings: Warnings, mut errors: Errors) -> uf<String> {
    // ensuring path exists
    let file_path: PathType =
        get_file_path(errors.clone(), warnings, cmd.get_one::<PathBuf>("path")).resolve();

    // Changing owner ship of the file
    let (uid, gid) = get_id();
    set_file_ownership(&file_path.to_path_buf(), uid, gid);

    // Pusing commands to the array
    let mut command_data: Vec<String> = vec![];
    command_data.push(Actions::EncryptData.to_string());
    command_data.push(
        cmd.get_one::<String>("owner")
            .unwrap_or(&String::from("system"))
            .to_owned(),
    );
    command_data.push(
        cmd.get_one::<String>("name")
            .unwrap_or(&String::from("lost"))
            .to_string(),
    );
    command_data.push(file_path.to_string());

    // Creating the message
    let message: String = create_message(command_data);

    // Sending the message
    return match send_command(message) {
        Ok(d) => uf::new(Ok(d)),
        Err(e) => {
            errors.0.push(e);
            uf::new(Err(errors))
        }
    };
}

fn remove_file(cmd: clap::ArgMatches, _warnings: Warnings, mut errors: Errors) -> uf<String> {
    let mut command_data: Vec<String> = vec![];
    command_data.push(Actions::RemoveFile.to_string());
    command_data.push(
        cmd.get_one::<String>("owner")
            .unwrap_or(&String::from("system"))
            .to_owned(),
    );
    command_data.push(
        cmd.get_one::<String>("name")
            .unwrap_or(&String::from("lost"))
            .to_string(),
    );

    let message: String = create_message(command_data);

    match send_command(message) {
        Ok(d) => uf::new(Ok(d)),
        Err(e) => {
            errors.0.push(e);
            uf::new(Err(errors))
        }
    }
}

fn decrypt_file(cmd: clap::ArgMatches, _warnings: Warnings, mut errors: Errors) -> uf<String> {
    let mut command_data: Vec<String> = vec![];
    command_data.push(Actions::DecryptData.to_string());
    command_data.push(
        cmd.get_one::<String>("owner")
            .unwrap_or(&String::from("system"))
            .to_owned(),
    );
    command_data.push(
        cmd.get_one::<String>("name")
            .unwrap_or(&String::from("lost"))
            .to_string(),
    );

    let message: String = create_message(command_data);

    match send_command(message) {
        Ok(d) => {
            let bytes: String = d.data.replace("\0", "");
            let bytes_string: String = convert_to_string(bytes.as_bytes());
            let paths: Vec<&str> = bytes_string.split('-').collect();
            use std::fs;
            let data = fs::read_to_string(&paths[0]);
            match data {
                Ok(d) => {
                    output("GREEN", &d);
                    return uf::new(Ok(OkWarning {
                        data: PathType::Str(paths[0].into()).to_string(),
                        warning: _warnings,
                    }));
                }
                Err(_) => {
                    return uf::new(Ok(OkWarning {
                        data: PathType::Str(paths[0].into()).to_string(),
                        warning: _warnings,
                    }));
                }
            }
        }
        Err(e) => {
            errors.0.push(e);
            return uf::new(Err(errors));
        }
    }
}

fn decrypt_text(cmd: clap::ArgMatches, _warnings: Warnings, mut errors: Errors) -> uf<String> {
    let data: String = cmd
        .get_one::<String>("data")
        .unwrap_or(&hex::encode(String::from("Invalid Command")))
        .to_string();

    let mut command_data: Vec<String> = vec![];
    command_data.push(Actions::DecryptText.to_string());
    command_data.push(data.to_string());

    let message: String = create_message(command_data);

    match send_command(message) {
        Ok(mut d) => {
            d.data = String::from_utf8(
                hex::decode(d.data.replace("\0", "").as_bytes())
                    .unwrap_or(String::from("Corrupted").as_bytes().to_vec()),
            )
            .unwrap_or("Not String".to_owned());
            return uf::new(Ok(d));
        }
        Err(e) => {
            errors.0.push(e);
            uf::new(Err(errors))
        }
    }
}

fn encrypt_text(cmd: clap::ArgMatches, _warnings: Warnings, mut errors: Errors) -> uf<String> {
    let data: String = cmd
        .get_one::<String>("data")
        .unwrap_or(&hex::encode(String::from("Invalid Command")))
        .to_string();

    let mut command_data: Vec<String> = vec![];
    command_data.push(Actions::EncryptText.to_string());
    command_data.push(data.to_string());

    let message: String = create_message(command_data);

    match send_command(message) {
        Ok(d) => uf::new(Ok(d)),
        Err(e) => {
            errors.0.push(e);
            uf::new(Err(errors))
        }
    }
}

fn get_file_path(
    mut errors: Errors,
    _warnings: Warnings,
    option_path_ref: Option<&PathBuf>,
) -> uf<PathType> {
    let err = match option_path_ref {
        Some(d) => match d.to_path_buf().canonicalize() {
            Ok(d) => {
                let result = OkWarning {
                    data: PathType::PathBuf(d),
                    warning: Warnings::new(Vec::new()),
                };
                return uf::new(Ok(result));
            }
            Err(err) => RecsError::new_details(RecsErrorType::InvalidFile, &format!("{}", err)),
        },
        None => RecsError::new(RecsErrorType::InvalidFile),
    };
    errors.0.push(err);
    return uf::new(Err(Errors::new(Vec::new())));
}

fn set_file_ownership(path: &PathBuf, uid: Uid, gid: Gid) {
    chown(path, Some(uid), Some(gid)).expect("Failed to set file ownership");
}
