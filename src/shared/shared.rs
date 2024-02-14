use nix::unistd::{Gid, Uid};
use users::{Groups, Users, UsersCache};

pub fn get_id() -> (Uid, Gid) {
    let user_cache: UsersCache = UsersCache::new();
    let dusa_uid = user_cache.get_user_by_name("dusa").unwrap().uid();
    let dusa_gid = user_cache.get_group_by_name("dusa").unwrap().gid();

    (Uid::from_raw(dusa_uid), Gid::from_raw(dusa_gid))
}

// Helper function to convert bytes to a string
pub fn convert_to_string(bytes: &[u8]) -> String {
    let decoded_bytes = hex::decode(bytes).unwrap();
    let string = String::from_utf8(decoded_bytes).unwrap();
    string
}