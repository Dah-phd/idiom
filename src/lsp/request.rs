use std::path::Path;

use anyhow::{anyhow, Result};
use lsp_types::{
    request::{HoverRequest, Initialize, References, SignatureHelpRequest},
    ClientCapabilities, HoverClientCapabilities, HoverParams, InitializeParams, MarkupKind, PartialResultParams,
    Position, ReferenceClientCapabilities, ReferenceContext, ReferenceParams, SignatureHelpClientCapabilities,
    SignatureHelpParams, TextDocumentClientCapabilities, TextDocumentIdentifier, TextDocumentPositionParams,
    TextDocumentSyncClientCapabilities, Url, WorkDoneProgressParams, WorkspaceClientCapabilities, WorkspaceFolder,
};
use serde::Serialize;
use serde_json::to_string;

use super::as_url;

#[derive(Serialize)]
pub struct LSPRequest<T>
where
    T: lsp_types::request::Request,
    T::Params: serde::Serialize,
    T::Result: serde::de::DeserializeOwned,
{
    jsonrpc: String,
    pub id: usize,
    pub method: &'static str,
    params: T::Params,
}

impl<T> LSPRequest<T>
where
    T: lsp_types::request::Request,
    T::Params: serde::Serialize,
    T::Result: serde::de::DeserializeOwned,
{
    pub fn with(id: usize, params: T::Params) -> Self {
        Self { jsonrpc: String::from("2.0"), id, method: <T as lsp_types::request::Request>::METHOD, params }
    }

    pub fn stringify(&self) -> Result<String> {
        let request_msg = to_string(self)?;
        let ser_req = format!("Content-Length: {}\r\n\r\n{}", request_msg.len(), request_msg);
        Ok(ser_req)
    }

    pub fn references(path: &Path, line: u32, char: u32) -> Option<LSPRequest<References>> {
        Some(LSPRequest::with(
            0,
            ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: Position::new(line, char),
                },
                context: ReferenceContext { include_declaration: true },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        ))
    }

    pub fn signature_help(path: &Path, line: u32, char: u32) -> Option<LSPRequest<SignatureHelpRequest>> {
        Some(LSPRequest::with(
            0,
            SignatureHelpParams {
                context: None,
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: Position::new(line, char),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        ))
    }

    pub fn hover(path: &Path, line: u32, char: u32) -> Option<LSPRequest<HoverRequest>> {
        Some(LSPRequest::with(
            0,
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: Position::new(line, char),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        ))
    }

    pub fn init_request() -> Result<LSPRequest<Initialize>> {
        let pwd_uri =
            format!("file:///{}", std::env::current_dir()?.as_os_str().to_str().ok_or(anyhow!("pwd conversion err"))?);
        let uri = Url::parse(&pwd_uri)?;
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
