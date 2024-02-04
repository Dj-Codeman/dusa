use std::os::unix::net::UnixStream;
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let mut stream = UnixStream::connect("/tmp/recs_server_socket")?;
    let command = "insert some_filename some_owner some_name";
    stream.write_all(command.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    println!("Server response: {:?}", String::from_utf8_lossy(&response));

    Ok(())
}