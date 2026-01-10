//! `error` specifies the domain errors that can occur.

use std::fmt::Formatter;

#[derive(Debug)]
/// ErrorKind listing possible error variations in subbers.
pub enum ErrorKind {
    InvalidInput(String),
    NotFound,
    Conflict,
    Internal,
}

/// Error is the core Error type encompassing all error details.
pub struct Error {
    /// kind is the Error variant.
    pub kind: ErrorKind,
    /// context contains any operator targeted messaging.
    pub context: Option<String>,
    /// source contains any downstream or dependency error that was encountered.
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            context: None,
            source: None,
        }
    }

    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_source<E>(mut self, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

    pub fn with_source_str(self, source: String) -> Self {
        let s = StringError(source);
        self.with_source(s)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static)) // coerce without + Send + Sync
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::Conflict => write!(f, "resource already exists"),
            ErrorKind::NotFound => write!(f, "resource not found"),
            ErrorKind::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            ErrorKind::Internal => write!(f, "internal error"),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.kind)
            .field("context", &self.context)
            .field("source", &self.source)
            .finish()
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
            ErrorKind::Internal => std::io::Error::other(e),
        }
    }
}

#[derive(Debug)]
struct StringError(String);

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StringError {}
