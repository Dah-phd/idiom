use super::as_url;
use crate::{lsp::LSPResult, workspace::CursorPosition};

use lsp_types as lsp;
use lsp_types::{
    request::{
        Completion, GotoDeclaration, GotoDeclarationParams, GotoDefinition, HoverRequest, Initialize, References,
        Rename, SemanticTokensFullRequest, SemanticTokensRangeRequest, SignatureHelpRequest,
    },
    CompletionItemKindCapability, CompletionParams, GotoDefinitionParams, HoverParams, Range, ReferenceContext,
    ReferenceParams, RenameParams, SemanticTokensParams, SemanticTokensRangeParams, SignatureHelpParams,
    TextDocumentIdentifier, TextDocumentPositionParams, TextDocumentSyncClientCapabilities, WorkspaceFolder,
};
use serde::Serialize;
use serde_json::to_string;
use std::path::Path;

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

    pub fn stringify(&self) -> LSPResult<String> {
        let request_msg = to_string(self)?;
        let ser_req = format!("Content-Length: {}\r\n\r\n{}", request_msg.len(), request_msg);
        Ok(ser_req)
    }

    pub fn references(path: &Path, c: CursorPosition) -> Option<LSPRequest<References>> {
        Some(LSPRequest::with(
            0,
            ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                    position: c.into(),
                },
                context: ReferenceContext { include_declaration: false },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        ))
    }

    pub fn rename(path: &Path, c: CursorPosition, new_name: String) -> LSPResult<LSPRequest<Rename>> {
        Ok(LSPRequest::with(
            0,
            RenameParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: c.into(),
                },
                new_name,
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            },
        ))
    }

    pub fn semantics_full(path: &Path) -> LSPResult<LSPRequest<SemanticTokensFullRequest>> {
        Ok(LSPRequest::with(
            0,
            SemanticTokensParams {
                text_document: TextDocumentIdentifier::new(as_url(path)?),
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        ))
    }

    pub fn semantics_range(path: &Path, range: Range) -> LSPResult<LSPRequest<SemanticTokensRangeRequest>> {
        Ok(LSPRequest::with(
            0,
            SemanticTokensRangeParams {
                text_document: TextDocumentIdentifier::new(as_url(path)?),
                range,
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        ))
    }

    pub fn declaration(path: &Path, c: CursorPosition) -> Option<LSPRequest<GotoDeclaration>> {
        Some(LSPRequest::with(
            0,
            GotoDeclarationParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        ))
    }

    pub fn definition(path: &Path, c: CursorPosition) -> Option<LSPRequest<GotoDefinition>> {
        Some(LSPRequest::with(
            0,
            GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path).ok()?),
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        ))
    }

    #[inline]
    pub fn completion(path: &Path, c: CursorPosition) -> LSPResult<LSPRequest<Completion>> {
        Ok(LSPRequest::with(
            0,
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
                context: None,
            },
        ))
    }

    #[inline]
    pub fn signature_help(path: &Path, c: CursorPosition) -> LSPResult<LSPRequest<SignatureHelpRequest>> {
        Ok(LSPRequest::with(
            0,
            SignatureHelpParams {
                context: None,
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            },
        ))
    }

    #[inline]
    pub fn hover(path: &Path, c: CursorPosition) -> LSPResult<LSPRequest<HoverRequest>> {
        Ok(LSPRequest::with(
            0,
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            },
        ))
    }

    pub fn init_request() -> LSPResult<LSPRequest<Initialize>> {
        let uri = as_url(std::env::current_dir()?.as_path())?;
        Ok(LSPRequest::with(
            0,
            lsp::InitializeParams {
                workspace_folders: Some(vec![WorkspaceFolder { uri, name: "root".to_owned() }]),
                capabilities: lsp::ClientCapabilities {
                    workspace: Some(lsp::WorkspaceClientCapabilities { ..Default::default() }),
                    text_document: Some(lsp::TextDocumentClientCapabilities {
                        completion: Some(lsp::CompletionClientCapabilities {
                            completion_item: Some(lsp::CompletionItemCapability {
                                resolve_support: Some(lsp::CompletionItemCapabilityResolveSupport {
                                    properties: vec![
                                        String::from("documentation"),
                                        String::from("detail"),
                                        String::from("additionalTextEdits"),
                                    ],
                                }),
                                insert_replace_support: Some(true),
                                snippet_support: Some(true),
                                ..Default::default()
                            }),
                            completion_item_kind: Some(CompletionItemKindCapability { ..Default::default() }),
                            context_support: None, // additional context information Some(true)
                            ..Default::default()
                        }),
                        synchronization: Some(TextDocumentSyncClientCapabilities {
                            did_save: Some(true),
                            ..Default::default()
                        }),
                        hover: Some(lsp::HoverClientCapabilities {
                            content_format: Some(vec![lsp::MarkupKind::PlainText]),
                            ..Default::default()
                        }),
                        references: Some(lsp::ReferenceClientCapabilities::default()),
                        signature_help: Some(lsp::SignatureHelpClientCapabilities {
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
