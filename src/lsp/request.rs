use super::as_url;
use crate::{lsp::LSPResult, workspace::CursorPosition};

use lsp_types::{self as lsp, Uri};
use lsp_types::{
    request::{
        Completion, GotoDeclaration, GotoDeclarationParams, GotoDefinition, HoverRequest, Initialize, References,
        Rename, SemanticTokensFullRequest, SemanticTokensRangeRequest, SignatureHelpRequest,
    },
    CompletionParams, GotoDefinitionParams, HoverParams, Range, ReferenceContext, ReferenceParams, RenameParams,
    SemanticTokensParams, SemanticTokensRangeParams, SignatureHelpParams, TextDocumentIdentifier,
    TextDocumentPositionParams, WorkspaceFolder,
};
use serde::Serialize;
use serde_json::to_string;

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

    pub fn references(path: Uri, c: CursorPosition, id: i64) -> LSPRequest<References> {
        LSPRequest::with(
            id,
            ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(path),
                    position: c.into(),
                },
                context: ReferenceContext { include_declaration: false },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        )
    }

    pub fn rename(uri: Uri, c: CursorPosition, new_name: String, id: i64) -> LSPRequest<Rename> {
        LSPRequest::with(
            id,
            RenameParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: c.into(),
                },
                new_name,
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            },
        )
    }

    pub fn semantics_full(uri: Uri, id: i64) -> LSPRequest<SemanticTokensFullRequest> {
        LSPRequest::with(
            id,
            SemanticTokensParams {
                text_document: TextDocumentIdentifier { uri },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        )
    }

    pub fn semantics_range(uri: Uri, range: Range, id: i64) -> LSPRequest<SemanticTokensRangeRequest> {
        LSPRequest::with(
            id,
            SemanticTokensRangeParams {
                text_document: TextDocumentIdentifier { uri },
                range,
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        )
    }

    pub fn declaration(uri: Uri, c: CursorPosition, id: i64) -> LSPRequest<GotoDeclaration> {
        LSPRequest::with(
            id,
            GotoDeclarationParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(uri),
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        )
    }

    pub fn definition(uri: Uri, c: CursorPosition, id: i64) -> LSPRequest<GotoDefinition> {
        LSPRequest::with(
            id,
            GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        )
    }

    #[inline]
    pub fn completion(uri: Uri, c: CursorPosition, id: i64) -> LSPRequest<Completion> {
        LSPRequest::with(
            id,
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
                context: None,
            },
        )
    }

    #[inline]
    pub fn signature_help(uri: Uri, c: CursorPosition, id: i64) -> LSPRequest<SignatureHelpRequest> {
        LSPRequest::with(
            id,
            SignatureHelpParams {
                context: None,
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            },
        )
    }

    #[inline]
    pub fn hover(uri: Uri, c: CursorPosition, id: i64) -> LSPRequest<HoverRequest> {
        LSPRequest::with(
            id,
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: c.into(),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            },
        )
    }

    pub fn init_request() -> LSPResult<LSPRequest<Initialize>> {
        let uri = as_url(std::env::current_dir()?.as_path());
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
                            completion_item_kind: Some(lsp::CompletionItemKindCapability { ..Default::default() }),
                            context_support: None, // additional context information Some(true)
                            ..Default::default()
                        }),
                        synchronization: Some(lsp::TextDocumentSyncClientCapabilities {
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
                    general: Some(lsp::GeneralClientCapabilities {
                        position_encodings: Some(vec![
                            lsp::PositionEncodingKind::UTF32, // preffered - but all are supported
                            lsp::PositionEncodingKind::UTF16,
                            lsp::PositionEncodingKind::UTF8,
                        ]),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
    }
}
