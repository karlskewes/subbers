//! `error` specifies the domain errors that can occur.

use std::fmt::Formatter;

#[derive(Debug)]
pub enum ErrorKind {
    InvalidInput(String),
    NotFound,
    Conflict,
    Internal(String),
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::Conflict => write!(f, "resource already exists"),
            ErrorKind::NotFound => write!(f, "resource not found"),
            ErrorKind::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            ErrorKind::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> std::io::Error {
        match e.kind {
            ErrorKind::Conflict => {
                std::io::Error::new(std::io::ErrorKind::AlreadyExists, "resource already exists")
            }
            ErrorKind::NotFound => {
                std::io::Error::new(std::io::ErrorKind::NotFound, "resource not found")
            }
            ErrorKind::InvalidInput(msg) => {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, msg)
            }
            ErrorKind::Internal(msg) => std::io::Error::other(msg),
        }
    }
}
