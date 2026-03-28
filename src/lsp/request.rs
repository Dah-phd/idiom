use super::as_url;
use crate::{cursor::CursorPosition, lsp::LSPResult};
use lsp_types::{self as lsp, Uri};
use lsp_types::{
    request::{
        Completion, Formatting, GotoDeclaration, GotoDeclarationParams, GotoDefinition, HoverRequest, Initialize,
        References, Rename, SemanticTokensFullRequest, SemanticTokensRangeRequest, SignatureHelpRequest,
    },
    CompletionParams, DocumentFormattingParams, GotoDefinitionParams, HoverParams, Range, ReferenceContext,
    ReferenceParams, RenameParams, SemanticTokensParams, SemanticTokensRangeParams, SignatureHelpParams,
    TextDocumentIdentifier, TextDocumentPositionParams, WorkspaceFolder,
};
use serde::Serialize;
use serde_json::{to_string, Value as Jval};

#[derive(Serialize, Debug)]
pub struct LSPRequest<T>
where
    T: lsp_types::request::Request,
    T::Params: serde::Serialize,
    T::Result: serde::de::DeserializeOwned,
{
    jsonrpc: String,
    id: i64,
    method: &'static str,
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

    pub fn formatting(uri: Uri, indent: usize, id: i64) -> LSPRequest<Formatting> {
        LSPRequest::with(
            id,
            DocumentFormattingParams {
                text_document: TextDocumentIdentifier { uri },
                options: lsp::FormattingOptions {
                    tab_size: indent as u32,
                    insert_spaces: true,
                    trim_final_newlines: Some(false),
                    trim_trailing_whitespace: Some(true),
                    insert_final_newline: Some(true),
                    properties: std::collections::HashMap::default(),
                },
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
}

impl LSPRequest<Initialize> {
    pub fn init_request() -> LSPResult<LSPRequest<Initialize>> {
        let uri = as_url(std::env::current_dir()?.as_path());
        Ok(LSPRequest::with(0, default_init_params(uri)))
    }

    pub fn init_request_with_mods(init_cfg: serde_json::Map<String, Jval>) -> LSPResult<LSPRequest<Initialize>> {
        use serde_json::{map::Entry, Value};

        let uri = as_url(std::env::current_dir()?.as_path());
        let Ok(values) = serde_json::to_value(default_init_params(uri)) else {
            return Self::init_request();
        };
        let Value::Object(mut map) = values else {
            return Self::init_request();
        };
        for (key, val) in init_cfg.into_iter() {
            match map.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(val);
                }
                Entry::Occupied(mut entry) => {
                    try_merge_values(entry.get_mut(), val);
                }
            };
        }
        let Ok(params) = serde_json::from_value::<lsp::InitializeParams>(Value::Object(map)) else {
            return Self::init_request();
        };
        Ok(LSPRequest::with(0, params))
    }
}

fn try_merge_values(target: &mut Jval, source: Jval) {
    use serde_json::{map::Entry, Value as Jval};
    if !target.is_object() || !source.is_object() {
        *target = source;
        return;
    }

    let (Jval::Object(table), Jval::Object(map)) = (source, target) else {
        return;
    };

    for (key, val) in table.into_iter() {
        match map.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(val);
            }
            Entry::Occupied(mut entry) => {
                try_merge_values(entry.get_mut(), val);
            }
        };
    }
}

fn default_init_params(uri: Uri) -> lsp::InitializeParams {
    lsp::InitializeParams {
        workspace_folders: Some(vec![WorkspaceFolder { uri, name: "root".to_owned() }]),
        capabilities: lsp::ClientCapabilities {
            workspace: Some(lsp::WorkspaceClientCapabilities { ..Default::default() }),
            text_document: Some(lsp::TextDocumentClientCapabilities {
                formatting: Some(lsp::DocumentFormattingClientCapabilities::default()),
                semantic_tokens: Some(lsp::SemanticTokensClientCapabilities {
                    overlapping_token_support: Some(false),
                    multiline_token_support: Some(false),
                    augments_syntax_tokens: Some(false),
                    ..Default::default()
                }),
                completion: Some(lsp::CompletionClientCapabilities {
                    completion_item: Some(lsp::CompletionItemCapability {
                        insert_replace_support: Some(true),
                        snippet_support: Some(true),
                        ..Default::default()
                    }),
                    completion_item_kind: Some(lsp::CompletionItemKindCapability { ..Default::default() }),
                    context_support: Some(true),
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
                                                      // lsp::PositionEncodingKind::UTF16,
                                                      // lsp::PositionEncodingKind::UTF8,
                ]),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod test {
    use super::LSPRequest;
    use crate::configs::{EditorConfigs, FileType};

    #[test]
    fn test_cfg_upgradee() {
        let editor_cfg = EditorConfigs::default();
        let (_, mods) = editor_cfg.derive_lsp(&FileType::Python).unwrap();
        let mut init = LSPRequest::init_request_with_mods(mods.unwrap()).unwrap();
        let mut init_default = LSPRequest::init_request().unwrap();
        let tokens = init.params.capabilities.text_document.as_mut().unwrap().semantic_tokens.take();
        let tokens_default = init_default.params.capabilities.text_document.as_mut().unwrap().semantic_tokens.take();
        assert_eq!(init.params, init_default.params);
        assert!(tokens.is_none());
        assert!(tokens_default.is_some());
        assert_ne!(tokens, tokens_default);
    }
}
