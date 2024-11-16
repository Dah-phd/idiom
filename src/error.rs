use crate::lsp::LSPError;
use std::fmt::Display;
use std::io::ErrorKind;
use thiserror::Error;
pub type IdiomResult<T> = Result<T, IdiomError>;

#[derive(Error, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum IdiomError {
    LSP(#[from] LSPError),
    IOError(#[from] std::io::Error),
    GeneralError(String),
}

impl IdiomError {
    pub fn any(message: impl Into<String>) -> Self {
        Self::GeneralError(message.into())
    }

    pub fn io_exists(message: impl Into<String>) -> Self {
        Self::IOError(std::io::Error::new(ErrorKind::AlreadyExists, message.into()))
    }

    pub fn io_other(message: impl Into<String>) -> Self {
        Self::IOError(std::io::Error::new(ErrorKind::Other, message.into()))
    }

    pub fn io_not_found(message: impl Into<String>) -> Self {
        Self::IOError(std::io::Error::new(ErrorKind::NotFound, message.into()))
    }
}

impl Display for IdiomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LSP(err) => {
                f.write_str("LSP - ")?;
                Display::fmt(err, f)
            }
            Self::IOError(message) => f.write_fmt(format_args!("IO Err: {message}")),
            Self::GeneralError(message) => f.write_str(message),
        }
    }
}
