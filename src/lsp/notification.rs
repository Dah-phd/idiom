use crate::{configs::FileType, lsp::LSPResult};

use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidRenameFiles, DidSaveTextDocument,
        Notification,
    },
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    FileRename, RenameFilesParams, TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, Uri,
    VersionedTextDocumentIdentifier,
};
use serde::Serialize;
use serde_json::to_string;

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

    pub fn stringify(&self) -> LSPResult<String> {
        let request_msg = to_string(self)?;
        let ser_req = format!("Content-Length: {}\r\n\r\n{}", request_msg.len(), request_msg);
        Ok(ser_req)
    }

    pub fn rename_file(old_uri: Uri, new_uri: Uri) -> LSPResult<LSPNotification<DidRenameFiles>> {
        Ok(LSPNotification::with(RenameFilesParams {
            files: vec![FileRename { old_uri: to_string(&old_uri)?, new_uri: to_string(&new_uri)? }],
        }))
    }

    pub fn file_did_change(
        uri: Uri,
        version: i32,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    ) -> LSPNotification<DidChangeTextDocument> {
        LSPNotification::with(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier::new(uri, version),
            content_changes,
        })
    }

    pub fn file_did_open(uri: Uri, file_type: FileType, content: String) -> LSPNotification<DidOpenTextDocument> {
        LSPNotification::with(DidOpenTextDocumentParams {
            text_document: TextDocumentItem { uri, language_id: String::from(file_type), version: 0, text: content },
        })
    }

    pub fn file_did_save(uri: Uri, content: String) -> LSPNotification<DidSaveTextDocument> {
        LSPNotification::with(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text: Some(content),
        })
    }

    pub fn file_did_close(uri: Uri) -> LSPNotification<DidCloseTextDocument> {
        LSPNotification::with(DidCloseTextDocumentParams { text_document: TextDocumentIdentifier { uri } })
    }
}
