use std::path::Path;

use anyhow::Result;
use lsp_types::{
    request::{
        Completion, HoverRequest, Initialize, References, Rename, SemanticTokensFullRequest,
        SemanticTokensRangeRequest, SignatureHelpRequest,
    },
    ClientCapabilities, CompletionParams, HoverClientCapabilities, HoverParams, InitializeParams, MarkupKind,
    PartialResultParams, Position, Range, ReferenceClientCapabilities, ReferenceContext, ReferenceParams, RenameParams,
    SemanticTokensParams, SemanticTokensRangeParams, SignatureHelpClientCapabilities, SignatureHelpParams,
    TextDocumentClientCapabilities, TextDocumentIdentifier, TextDocumentPositionParams,
    TextDocumentSyncClientCapabilities, WorkDoneProgressParams, WorkspaceClientCapabilities, WorkspaceFolder,
};
use serde::Serialize;
use serde_json::to_string;

use crate::components::editor::CursorPosition;

use super::as_url;

#[derive(Serialize)]
pub struct LSPRequest<T>
where
    T: lsp_types::request::Request,
    T::Params: serde::Serialize,
    T::Result: serde::de::DeserializeOwned,
{
    jsonrpc: String,
    pub id: i64,
    pub method: &'static str,
    params: T::Params,
}

impl<T> LSPRequest<T>
where
    T: lsp_types::request::Request,
    T::Params: serde::Serialize,
    T::Result: serde::de::DeserializeOwned,
{
    pub fn with(id: i64, params: T::Params) -> Self {
        Self { jsonrpc: String::from("2.0"), id, method: <T as lsp_types::request::Request>::METHOD, params }
    }

    pub fn stringify(&self) -> Result<String> {
        let request_msg = to_string(self)?;
        let ser_req = format!("Content-Length: {}\r\n\r\n{}", request_msg.len(), request_msg);
        Ok(ser_req)
    }

    pub fn references(path: &Path, c: &CursorPosition) -> Option<LSPRequest<References>> {
        Some(LSPRequest::with(
            0,
            ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                    position: c.into(),
                },
                context: ReferenceContext { include_declaration: true },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        ))
    }

    pub fn rename(path: &Path, c: &CursorPosition, new_name: String) -> Option<LSPRequest<Rename>> {
        Some(LSPRequest::with(
            0,
            RenameParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                    position: c.into(),
                },
                new_name,
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        ))
    }

    pub fn semantics_full(path: &Path) -> Option<LSPRequest<SemanticTokensFullRequest>> {
        Some(LSPRequest::with(
            0,
            SemanticTokensParams {
                text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        ))
    }

    pub fn semantics_range(
        path: &Path,
        from: &CursorPosition,
        to: &CursorPosition,
    ) -> Option<LSPRequest<SemanticTokensRangeRequest>> {
        Some(LSPRequest::with(
            0,
            SemanticTokensRangeParams {
                text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                range: Range::new(from.into(), to.into()),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        ))
    }

    pub fn completion(path: &Path, c: &CursorPosition) -> Option<LSPRequest<Completion>> {
        Some(LSPRequest::with(
            0,
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                    position: c.into(),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            },
        ))
    }

    pub fn signature_help(path: &Path, line: u32, char: u32) -> Option<LSPRequest<SignatureHelpRequest>> {
        Some(LSPRequest::with(
            0,
            SignatureHelpParams {
                context: None,
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                    position: Position::new(line, char),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        ))
    }

    pub fn hover(path: &Path, c: &CursorPosition) -> Option<LSPRequest<HoverRequest>> {
        Some(LSPRequest::with(
            0,
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                    position: c.into(),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        ))
    }

    pub fn init_request() -> Result<LSPRequest<Initialize>> {
        let uri = as_url(std::env::current_dir()?.as_path())?;
        Ok(LSPRequest::with(
            0,
            InitializeParams {
                workspace_folders: Some(vec![WorkspaceFolder { uri, name: "root".to_owned() }]),
                capabilities: ClientCapabilities {
                    workspace: Some(WorkspaceClientCapabilities { ..Default::default() }),
                    text_document: Some(TextDocumentClientCapabilities {
                        synchronization: Some(TextDocumentSyncClientCapabilities {
                            will_save: Some(true),
                            did_save: Some(true),
                            ..Default::default()
                        }),
                        hover: Some(HoverClientCapabilities {
                            content_format: Some(vec![MarkupKind::PlainText]),
                            ..Default::default()
                        }),
                        references: Some(ReferenceClientCapabilities::default()),
                        signature_help: Some(SignatureHelpClientCapabilities {
                            context_support: Some(true),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
    }
}
