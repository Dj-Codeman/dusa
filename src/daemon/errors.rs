use std::fmt;
use std::io;
use std::num;

#[derive(Debug)]
pub enum ServerError {
    IOError(io::Error),
    ParseError(num::ParseIntError),
    _CustomError(String),
    // Add more error variants as needed
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::IOError(err) => write!(f, "IO Error: {}", err),
            ServerError::ParseError(err) => write!(f, "Parse Error: {}", err),
            ServerError::_CustomError(msg) => write!(f, "Custom Error: {}", msg),
            // Add more error descriptions here
        }
    }
}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> Self {
        ServerError::IOError(err)
    }
}

impl From<num::ParseIntError> for ServerError {
    fn from(err: num::ParseIntError) -> Self {
        ServerError::ParseError(err)
    }
}