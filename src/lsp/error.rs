use std::fmt::Display;
use thiserror::Error;

use crate::lsp::lsp_stream;

use super::client::Payload;
pub type LSPResult<T> = Result<T, LSPError>;

#[derive(Error, Debug)]
pub enum LSPError {
    UrlPathError(#[from] url::ParseError),
    ResponseError(String),
    InternalError(String),
    JsonError(#[from] serde_json::error::Error),
    SendError(#[from] tokio::sync::mpsc::error::SendError<Payload>),
    ServerCapability(String),
    IOError(#[from] std::io::Error),
    JsonRCPStderr(#[from] lsp_stream::RCPError),
    Null,
}

impl LSPError {
    #[inline]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }

    #[inline]
    pub fn missing_capability(message: impl Into<String>) -> Self {
        Self::ServerCapability(message.into())
    }
}

impl Display for LSPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => Ok(()),
            Self::InternalError(message) => f.write_fmt(format_args!("LSP Internal Error: {message}")),
            Self::ResponseError(message) => f.write_fmt(format_args!("LSP Responde with error: {message}")),
            Self::JsonRCPStderr(err) => {
                f.write_str("LSP ERR message: ")?;
                Display::fmt(err, f)
            }
            Self::UrlPathError(err) => {
                f.write_str("LSP Error - failed to parse file url: ")?;
                Display::fmt(err, f)
            }
            Self::JsonError(err) => {
                f.write_str("Internal Error on JSON parsing: ")?;
                Display::fmt(err, f)
            }
            Self::SendError(err) => {
                f.write_str("LSP SendError: ")?;
                Display::fmt(err, f)
            }
            Self::ServerCapability(message) => {
                f.write_fmt(format_args!("LSP Error: Server is unable to process {}.", message))
            }
            Self::IOError(err) => {
                f.write_str("IO Error: ")?;
                Display::fmt(err, f)
            }
        }
    }
}
