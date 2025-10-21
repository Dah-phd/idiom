mod enriched;
mod styler;
mod tokens;
mod utils; // support TS and JS

use tokens::bash::BashToken;
use tokens::generic::GenericToken;
/// tokens
pub use tokens::init_local_tokens;
use tokens::json::JsonValue;
use tokens::lobster::Pincer;
use tokens::python::PyToken;
use tokens::rust::Rustacean;
use tokens::ts::TSToken;
use tokens::{Definitions, LangStream, PositionedToken};

pub use enriched::build_with_enrichment;
pub use styler::Highlighter;
pub use utils::create_semantic_capabilities;
use utils::{full_tokens, partial_tokens, swap_content};

use super::{payload::Payload, LSPError, LSPResponse, LSPResult, Responses};
use crate::{configs::FileType, workspace::CursorPosition};

use lsp_types::{
    notification::{DidChangeTextDocument, DidOpenTextDocument, Notification},
    SemanticTokens, SemanticTokensRangeResult, SemanticTokensResult,
};
use serde_json::{from_str, Value};
use std::sync::Arc;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};

/// Not fully blowns LSP - but struct processing tokens better, giving basic utils, like semantics, autocomplete, rename
#[derive(Default)]
struct LocalLSP<T: LangStream> {
    definitions: Definitions,
    text: Vec<String>,
    tokens: Vec<Vec<PositionedToken<T>>>,
    responses: Arc<Responses>,
}

pub fn start_lsp_handler(
    rx: UnboundedReceiver<Payload>,
    file_type: FileType,
    responses: Arc<Responses>,
) -> JoinHandle<LSPResult<()>> {
    match file_type {
        FileType::Python => tokio::task::spawn(async move { LocalLSP::<PyToken>::run(rx, responses).await }),
        FileType::Lobster => tokio::task::spawn(async move { LocalLSP::<Pincer>::run(rx, responses).await }),
        FileType::Rust => tokio::task::spawn(async move { LocalLSP::<Rustacean>::run(rx, responses).await }),
        FileType::JavaScript => tokio::task::spawn(async move { LocalLSP::<TSToken>::run(rx, responses).await }),
        FileType::TypeScript => tokio::task::spawn(async move { LocalLSP::<TSToken>::run(rx, responses).await }),
        FileType::Json => tokio::task::spawn(async move { LocalLSP::<JsonValue>::run(rx, responses).await }),
        FileType::Shell => tokio::task::spawn(async move { LocalLSP::<BashToken>::run(rx, responses).await }),
        _ => tokio::task::spawn(async move { LocalLSP::<GenericToken>::run(rx, responses).await }),
    }
}

impl<T: LangStream> LocalLSP<T> {
    async fn run(mut rx: UnboundedReceiver<Payload>, responses: Arc<Responses>) -> LSPResult<()> {
        let mut lsp = Self::new(responses);
        while let Some(payload) = rx.recv().await {
            lsp.parase_payload(payload)?;
        }
        Ok(())
    }

    fn new(responses: Arc<Responses>) -> Self {
        Self { definitions: T::init_definitions(), text: Vec::new(), tokens: Vec::new(), responses }
    }

    fn parase_payload(&mut self, payload: Payload) -> LSPResult<()> {
        match payload {
            Payload::Direct(data) => {
                self.direct_parsing(data)?;
            }
            Payload::Tokens(_, id) => {
                let tokens =
                    SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data: full_tokens(&self.tokens) });
                self.responses.lock().unwrap().insert(id, LSPResponse::Tokens(tokens));
            }
            Payload::PartialTokens(_, range, id, max_lines) => {
                let start = CursorPosition::from(range.start);
                let end = CursorPosition::from(range.end);
                let tokens = SemanticTokensRangeResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: partial_tokens(&self.tokens, start, end),
                });
                self.responses.lock().unwrap().insert(id, LSPResponse::TokensPartial { result: tokens, max_lines });
            }
            Payload::Sync(.., change_event) => {
                for change in change_event {
                    let range = change.range.unwrap();
                    let from = CursorPosition::from(range.start);
                    let to = CursorPosition::from(range.end);
                    let clip = change.text;
                    swap_content(&mut self.text, &clip, from, to);
                }
                T::parse(self.text.iter().map(|t| t.as_str()), &mut self.tokens, PositionedToken::<T>::utf32);
            }
            Payload::FullSync(.., full_text) => {
                self.text = full_text.split('\n').map(ToOwned::to_owned).collect();
                T::parse(self.text.iter().map(|t| t.as_str()), &mut self.tokens, PositionedToken::<T>::utf32);
            }
            Payload::Completion(_, cursor, id, line) => {
                let items = self.definitions.to_completions(&self.tokens);
                self.responses.lock().unwrap().insert(id, LSPResponse::Completion(items, line, cursor));
            }
            _ => {}
        };
        Ok(())
    }

    fn direct_parsing(&mut self, data: String) -> Result<(), LSPError> {
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
            DidChangeTextDocument::METHOD => Ok(()),
            _ => Ok(()),
        }
    }

    fn file_did_open(&mut self, mut val: Value) -> Option<()> {
        self.text.clear();
        let params = val.as_object_mut()?.get_mut("params")?;
        let documet = params.as_object_mut()?.get_mut("textDocument")?;
        let text = documet.as_object_mut()?.get("text")?.as_str()?;
        self.text = text.split('\n').map(ToOwned::to_owned).collect();
        T::parse(self.text.iter().map(|t| t.as_str()), &mut self.tokens, PositionedToken::<T>::utf32);
        Some(())
    }
}
