//! `error` specifies the domain errors that can occur.

use std::fmt::Formatter;

#[derive(Debug)]
pub enum Error {
    InvalidInput(String),
    NotFound,
    Conflict,
    Internal(String),
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict => write!(f, "resource already exists"),
            Self::NotFound => write!(f, "resource not found"),
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

// Convert from an owned `Game`, avoiding clone in caller.
impl From<Error> for std::io::Error {
    fn from(e: Error) -> std::io::Error {
        match e {
            Error::Conflict => {
                std::io::Error::new(std::io::ErrorKind::AlreadyExists, "resource already exists")
            }
            Error::NotFound => {
                std::io::Error::new(std::io::ErrorKind::NotFound, "resource not found")
            }
            Error::InvalidInput(msg) => std::io::Error::new(std::io::ErrorKind::InvalidInput, msg),
            Error::Internal(msg) => std::io::Error::other(msg),
        }
    }
}
