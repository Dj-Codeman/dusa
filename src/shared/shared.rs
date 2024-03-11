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

pub fn nokay_val() -> String {
    hex::encode(String::from("Invalid Command"))
}
