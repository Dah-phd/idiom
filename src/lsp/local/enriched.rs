use std::{collections::HashMap, sync::Arc};

use crate::lsp::local::{
    generic::GenericToken,
    lobster::Pincer,
    python::PyToken,
    utils::{full_tokens, partial_tokens, swap_content},
};
use crate::{
    configs::FileType,
    lsp::{
        local::{Definitions, LangStream, PositionedToken, Responses},
        payload::Payload,
        LSPError, LSPResult, Response,
    },
    workspace::CursorPosition,
};
use lsp_types::{
    notification::{DidOpenTextDocument, Notification},
    DidOpenTextDocumentParams, SemanticTokens, SemanticTokensRangeResult, SemanticTokensResult,
    TextDocumentContentChangeEvent, TextDocumentItem, Uri,
};
use serde_json::{from_str, from_value, to_value, Value};
use tokio::{io::AsyncWriteExt, process::ChildStdin, sync::mpsc::UnboundedReceiver, task::JoinHandle};

pub fn enrich_with_semantics(
    rx: UnboundedReceiver<Payload>,
    lsp_stdin: ChildStdin,
    file_type: FileType,
    responses: Arc<Responses>,
) -> JoinHandle<LSPResult<()>> {
    match file_type {
        FileType::Python => {
            tokio::task::spawn(async move { EnrichedLSP::<PyToken>::run_tokens(rx, lsp_stdin, responses).await })
        }
        FileType::Lobster => {
            tokio::task::spawn(async move { EnrichedLSP::<Pincer>::run_tokens(rx, lsp_stdin, responses).await })
        }
        _ => tokio::task::spawn(async move { EnrichedLSP::<GenericToken>::run_tokens(rx, lsp_stdin, responses).await }),
    }
}

/// Enrichment allows to imporve LSP capabilities by running process that will handle smaller process before the process
struct EnrichedLSP<T: LangStream> {
    documents: HashMap<Uri, DocumentData<T>>,
    definitions: Definitions,
    responses: Arc<Responses>,
}

impl<T: LangStream> EnrichedLSP<T> {
    async fn run_tokens(
        mut rx: UnboundedReceiver<Payload>,
        mut lsp_stdin: ChildStdin,
        responses: Arc<Responses>,
    ) -> LSPResult<()> {
        let mut lsp_wrapper = Self::new(responses);
        while let Some(payload) = rx.recv().await {
            if let Some(msg) = lsp_wrapper.pre_process(payload)? {
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            };
        }
        Ok(())
    }

    async fn run_with_sync_coersion(
        mut rx: UnboundedReceiver<Payload>,
        mut lsp_stdin: ChildStdin,
        responses: Arc<Responses>,
    ) -> LSPResult<()> {
        let mut lsp_wrapper = Self::new(responses);
        while let Some(payload) = rx.recv().await {
            if let Some(msg) = lsp_wrapper.pre_process(payload)? {
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            };
        }
        Ok(())
    }

    async fn run_with_autocomplete(
        mut rx: UnboundedReceiver<Payload>,
        mut lsp_stdin: ChildStdin,
        responses: Arc<Responses>,
    ) -> LSPResult<()> {
        Ok(())
    }

    async fn run_full(
        mut rx: UnboundedReceiver<Payload>,
        mut lsp_stdin: ChildStdin,
        responses: Arc<Responses>,
    ) -> LSPResult<()> {
        Ok(())
    }

    fn new(responses: Arc<Responses>) -> Self {
        Self { documents: HashMap::new(), definitions: T::init_definitions(), responses }
    }

    fn pre_process(&mut self, payload: Payload) -> LSPResult<Option<String>> {
        match payload {
            Payload::Direct(data) => {
                self.direct_parsing(&data)?;
                Ok(Some(data))
            }
            Payload::Tokens(uri, id) => {
                let data = match self.documents.get(&uri) {
                    Some(doc) => full_tokens(&doc.tokens),
                    None => vec![],
                };
                let tokens = SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data });
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(error) => Response { id, result: None, error: Some(Value::String(error.to_string())) },
                };
                self.responses.lock().unwrap().insert(id, response);
                Ok(None)
            }
            Payload::PartialTokens(uri, range, id, ..) => {
                let data = match self.documents.get(&uri) {
                    Some(doc) => partial_tokens(&doc.tokens, range),
                    None => vec![],
                };
                let tokens = SemanticTokensRangeResult::Tokens(SemanticTokens { result_id: None, data });
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(err) => Response { id, result: None, error: Some(Value::String(err.to_string())) },
                };
                self.responses.lock().unwrap().insert(id, response);
                Ok(None)
            }
            Payload::Sync(uri, version, change_event) => {
                if let Some(doc) = self.documents.get_mut(&uri) {
                    doc.sync(&change_event);
                };
                Ok(Payload::Sync(uri, version, change_event).try_stringify().ok())
            }
            Payload::FullSync(uri, version, full_text) => {
                if let Some(doc) = self.documents.get_mut(&uri) {
                    doc.full_sync(&full_text);
                };
                Ok(Payload::FullSync(uri, version, full_text).try_stringify().ok())
            }
            _ => Ok(payload.try_stringify().ok()),
        }
    }

    fn pre_process_sync_coersion(&mut self, payload: Payload) -> LSPResult<Option<String>> {
        match payload {
            Payload::Direct(data) => {
                self.direct_parsing(&data)?;
                Ok(Some(data))
            }
            Payload::Tokens(uri, id) => {
                let data = match self.documents.get(&uri) {
                    Some(doc) => full_tokens(&doc.tokens),
                    None => vec![],
                };
                let tokens = SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data });
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(error) => Response { id, result: None, error: Some(Value::String(error.to_string())) },
                };
                self.responses.lock().unwrap().insert(id, response);
                Ok(None)
            }
            Payload::PartialTokens(uri, range, id, ..) => {
                let data = match self.documents.get(&uri) {
                    Some(doc) => partial_tokens(&doc.tokens, range),
                    None => vec![],
                };
                let tokens = SemanticTokensRangeResult::Tokens(SemanticTokens { result_id: None, data });
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(err) => Response { id, result: None, error: Some(Value::String(err.to_string())) },
                };
                self.responses.lock().unwrap().insert(id, response);
                Ok(None)
            }
            Payload::Sync(uri, version, change_event) => {
                if let Some(doc) = self.documents.get_mut(&uri) {
                    let full_text = doc.sync_to_full_sync(&change_event);
                    Ok(Payload::FullSync(uri, version, full_text).try_stringify().ok())
                } else {
                    Ok(Payload::Sync(uri, version, change_event).try_stringify().ok())
                }
            }
            Payload::FullSync(uri, version, full_text) => {
                if let Some(doc) = self.documents.get_mut(&uri) {
                    doc.full_sync(&full_text);
                };
                Ok(Payload::FullSync(uri, version, full_text).try_stringify().ok())
            }
            _ => Ok(payload.try_stringify().ok()),
        }
    }

    fn direct_parsing(&mut self, data: &str) -> Result<(), LSPError> {
        let (_h, msg) = data.split_once("\r\n\r\n").ok_or_else(|| LSPError::internal("Message header not found!"))?;
        let val = from_str::<Value>(msg)?;
        match val
            .as_object()
            .ok_or_else(|| LSPError::internal("Unexpected message format!"))?
            .get("method")
            .and_then(|meth| meth.as_str())
            .ok_or_else(|| LSPError::internal("No method found ot message!"))?
        {
            DidOpenTextDocument::METHOD => self
                .file_did_open(val)
                .ok_or_else(|| LSPError::internal("Failed to parse didOpenDocument notification!")),
            _ => Ok(()),
        }
    }

    fn file_did_open(&mut self, val: Value) -> Option<()> {
        let DidOpenTextDocumentParams { text_document: TextDocumentItem { uri, text, .. } } =
            from_value::<DidOpenTextDocumentParams>(val.get("params").cloned()?).ok()?;
        self.documents.insert(uri, DocumentData::open(text));
        Some(())
    }
}

pub struct DocumentData<T: LangStream> {
    text: Vec<String>,
    tokens: Vec<Vec<PositionedToken<T>>>,
}

impl<T: LangStream> DocumentData<T> {
    fn open(text: String) -> Self {
        let text = text.split('\n').map(ToOwned::to_owned).collect();
        let mut doc = Self { text, tokens: vec![] };
        T::parse(doc.text.iter().map(|t| t.as_str()), &mut doc.tokens);
        doc
    }

    fn sync(&mut self, change_event: &[TextDocumentContentChangeEvent]) {
        for change in change_event {
            let range = change.range.unwrap();
            let from = CursorPosition::from(range.start);
            let to = CursorPosition::from(range.end);
            swap_content(&mut self.text, &change.text, from, to);
        }
        T::parse(self.text.iter().map(|t| t.as_str()), &mut self.tokens);
    }

    fn sync_to_full_sync(&mut self, change_event: &[TextDocumentContentChangeEvent]) -> String {
        self.sync(change_event);
        self.stringify()
    }

    fn full_sync(&mut self, new_text: &str) {
        self.text = new_text.split('\n').map(ToOwned::to_owned).collect();
        T::parse(self.text.iter().map(|t| t.as_str()), &mut self.tokens);
    }

    #[inline]
    pub fn stringify(&self) -> String {
        let mut text = self.text.iter().map(|l| l.as_str()).collect::<Vec<_>>().join("\n");
        text.push('\n');
        text
    }
}
