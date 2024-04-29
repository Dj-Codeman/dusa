use core::fmt;

use nix::unistd::{Gid, Uid};
use pretty::halt;
use users::{Groups, Users, UsersCache};

pub fn get_id() -> (Uid, Gid) {
    let user_cache: UsersCache = UsersCache::new();
    let dusa_uid = user_cache.get_user_by_name("dusa").unwrap().uid();
    let dusa_gid = user_cache.get_group_by_name("dusa").unwrap().gid();

    (Uid::from_raw(dusa_uid), Gid::from_raw(dusa_gid))
}

// Helper function to convert bytes to a string
pub fn convert_to_string(bytes: &[u8]) -> String {
    let data: Vec<u8> = match hex::decode(bytes) {
        Ok(d) => d,
        Err(e) => {
            halt(&format!(
                "The client created garbage data: {}",
                &e.to_string()
            ));

            panic!()
        }
    };

    let string = unsafe { String::from_utf8_unchecked(data) };
    string
}

#[allow(dead_code)]
pub fn okay_val(data: Option<Vec<String>>) -> String {
    match data {
        Some(response_data) => {
            let response_string: String = response_data.join("=");
            let hexed_response: String = hex::encode(response_string);
            hexed_response
        }
        None => hex::encode(String::from("Operation Performed successfully")),
    }
}

#[allow(dead_code)]
pub fn no_kay_val() -> String {
    hex::encode(String::from("Invalid Command"))
}

#[allow(dead_code)]
pub enum Actions {
    EncryptData,
    EncryptText,
    DecryptData,
    DecryptText,
    RemoveFile,
    Version,
    Ping,
    UpdateMaps
}

impl fmt::Display for Actions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self  {
            Actions::EncryptData => write!(f, "0x000"),
            Actions::EncryptText => write!(f, "0x001"),
            Actions::DecryptData => write!(f, "0x010"),
            Actions::DecryptText => write!(f, "0x011"),
            Actions::RemoveFile  => write!(f, "0x100"),
            Actions::Version     => write!(f, "0x101"),
            Actions::Ping        => write!(f, "0x110"),
            Actions::UpdateMaps  => write!(f, "0x111"),
        }   
    }
}

// // Struct to encrypt/encode and send back and forth from  the server
// pub struct Command {
//     command: Actions,
//     arguments: Vec<String>,
// }

// // Struct to capture the return data
// pub struct Result {}
