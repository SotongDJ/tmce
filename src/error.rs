use std::fmt;
use std::io;

#[derive(Debug)]
pub enum TmceError {
    Io(io::Error),
    Escape(String),
    InvalidOrder(String),
    InvalidUuid(String),
    Parse { line: usize, message: String },
    DuplicateMember(String),
    MissingMember(String),
    Other(String),
}

impl fmt::Display for TmceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TmceError::Io(e) => write!(f, "I/O error: {}", e),
            TmceError::Escape(s) => write!(f, "Escape error: {}", s),
            TmceError::InvalidOrder(s) => write!(f, "Invalid order digit: {}", s),
            TmceError::InvalidUuid(s) => write!(f, "Invalid UUID: {}", s),
            TmceError::Parse { line, message } => {
                write!(f, "Parse error at line {}: {}", line, message)
            }
            TmceError::DuplicateMember(id) => write!(f, "Duplicate member: {}", id),
            TmceError::MissingMember(id) => write!(f, "Missing member: {}", id),
            TmceError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl From<io::Error> for TmceError {
    fn from(e: io::Error) -> Self {
        TmceError::Io(e)
    }
}

impl std::error::Error for TmceError {}

pub type Result<T> = std::result::Result<T, TmceError>;
