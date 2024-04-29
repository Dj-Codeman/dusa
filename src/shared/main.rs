use recs::errors::{RecsError, RecsWarning, RecsWarningType};
use system::{del_file, ClonePath, PathType};
use warn::{Errors, OkWarning, UnifiedResult as uf, Warnings};

pub mod shared;
pub mod warn;

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
                        return uf::new(Ok(result))
                    },
                    Err(_) => {
                        w.0.push(RecsWarning::new(RecsWarningType::OutdatedVersion));
                        let result: OkWarning<PathType> = OkWarning {
                            data: socket_file,
                            warning: w,
                        };
                        return uf::new(Ok(result))
                    },
                },
                false => {
                    let result: OkWarning<PathType> = OkWarning {
                        data: socket_file,
                        warning: w,
                    };
                    return uf::new(Ok(result))
                },
            }
        },
        false => return uf::new(Ok(OkWarning{ data: socket_file, warning: w})),
    }
}
