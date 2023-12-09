use super::as_url;
use crate::configs::FileType;

use anyhow::Result;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Notification,
    },
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, VersionedTextDocumentIdentifier,
};
use serde::Serialize;
use serde_json::to_string;
use std::path::Path;

#[derive(Serialize)]
pub struct LSPNotification<T>
where
    T: lsp_types::notification::Notification,
    T::Params: serde::Serialize,
{
    jsonrpc: String,
    pub method: &'static str,
    params: T::Params,
}

impl<T> LSPNotification<T>
where
    T: lsp_types::notification::Notification,
    T::Params: serde::Serialize,
{
    pub fn with(params: T::Params) -> Self {
        Self { jsonrpc: String::from("2.0"), method: <T as Notification>::METHOD, params }
    }

    pub fn stringify(&self) -> Result<String> {
        let request_msg = to_string(self)?;
        let ser_req = format!("Content-Length: {}\r\n\r\n{}", request_msg.len(), request_msg);
        Ok(ser_req)
    }

    pub fn file_did_change(
        path: &Path,
        version: i32,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<LSPNotification<DidChangeTextDocument>> {
        Ok(LSPNotification::with(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier::new(as_url(path)?, version),
            content_changes,
        }))
    }

    pub fn file_did_open(
        path: &Path,
        file_type: &FileType,
        content: String,
    ) -> Result<LSPNotification<DidOpenTextDocument>> {
        Ok(LSPNotification::with(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: as_url(path)?,
                language_id: String::from(file_type),
                version: 0,
                text: content,
            },
        }))
    }

    pub fn file_did_save(path: &Path) -> Result<LSPNotification<DidSaveTextDocument>> {
        let content = std::fs::read_to_string(path)?;
        Ok(LSPNotification::with(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: as_url(path)? },
            text: Some(content),
        }))
    }

    pub fn file_did_close(path: &Path) -> Result<LSPNotification<DidCloseTextDocument>> {
        Ok(LSPNotification::with(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: as_url(path)? },
        }))
    }
}
