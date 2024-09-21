use lsp_types::InsertTextFormat;
mod enriched;
mod generic;
mod json;
mod lobster;
mod python;
mod rust;
mod text_editor;
mod ts; // support TS and JS
mod utils;

pub use enriched::enrich_with_semantics;
use rust::Rustacean;

use crate::lsp::local::{generic::GenericToken, python::PyToken};
use crate::lsp::{messages::Response, LSPError, LSPResult, Responses};
use crate::render::UTF8Safe;
use crate::syntax::theme::Theme;
use crate::syntax::tokens::set_tokens;
use crate::syntax::Legend;
use crate::workspace::line::EditorLine;
use crate::{configs::FileType, lsp::client::Payload, workspace::CursorPosition};
use json::JsonValue;
use lobster::Pincer;
use logos::{Logos, Span};
use lsp_types::{
    notification::{DidChangeTextDocument, DidOpenTextDocument, Notification},
    SemanticToken, SemanticTokens, SemanticTokensRangeResult, SemanticTokensResult,
};
use lsp_types::{CompletionItem, CompletionResponse};
use serde_json::{from_str, to_value, Value};
use std::collections::HashSet;
use std::fmt::{format, Debug};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
pub use utils::create_semantic_capabilities;
use utils::{full_tokens, partial_tokens, swap_content, NON_TOKEN_ID};

/// Trait to be implemented on the lang specific token, allowing parsing and deriving builtins
trait LangStream: Sized + Debug + PartialEq + Logos<'static> {
    fn init_definitions() -> Definitions {
        Definitions::default()
    }
    fn type_id(&self) -> u32;
    fn modifier(&self) -> u32;
    fn to_postioned(self, span: Span, text: &str) -> PositionedToken<Self> {
        // utf32 encoding
        let from = text[..span.start].char_len();
        let len = text[span.start..span.end].char_len();
        PositionedToken { from, len, token_type: self.type_id(), modifier: self.modifier(), lang_token: self }
    }
    fn objectify(&self) -> ObjType {
        ObjType::None
    }
    fn parse(text: &[String], tokens: &mut Vec<Vec<PositionedToken<Self>>>);

    fn init_tokens(content: &mut Vec<EditorLine>, theme: &Theme, file_type: FileType) {
        let text = content.iter().map(|l| l.content.to_string()).collect::<Vec<_>>();
        let mut tokens = Vec::new();
        Self::parse(&text, &mut tokens);
        let mut legend = Legend::default();
        legend.map_styles(file_type, theme, &create_semantic_capabilities());
        set_tokens(full_tokens(&tokens), &legend, theme, content);
    }
}

pub fn init_local_tokens(file_type: FileType, content: &mut Vec<EditorLine>, theme: &Theme) {
    match file_type {
        FileType::Rust => Rustacean::init_tokens(content, theme, file_type),
        FileType::Python => PyToken::init_tokens(content, theme, file_type),
        FileType::Lobster => Pincer::init_tokens(content, theme, file_type),
        _ => GenericToken::init_tokens(content, theme, file_type),
    }
}

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
        FileType::Json => tokio::task::spawn(async move { LocalLSP::<JsonValue>::run(rx, responses).await }),
        FileType::Rust => tokio::task::spawn(async move { LocalLSP::<Rustacean>::run(rx, responses).await }),
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
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(error) => Response { id, result: None, error: Some(Value::String(error.to_string())) },
                };
                self.responses.lock().unwrap().insert(id, response);
            }
            Payload::PartialTokens(_, range, id, ..) => {
                let tokens = SemanticTokensRangeResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: partial_tokens(&self.tokens, range),
                });
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(err) => Response { id, result: None, error: Some(Value::String(err.to_string())) },
                };
                self.responses.lock().unwrap().insert(id, response);
            }
            Payload::Sync(.., change_event) => {
                for change in change_event {
                    let range = change.range.unwrap();
                    let from = CursorPosition::from(range.start);
                    let to = CursorPosition::from(range.end);
                    let clip = change.text;
                    swap_content(&mut self.text, &clip, from, to);
                }
                T::parse(&self.text, &mut self.tokens);
            }
            Payload::FullSync(.., full_text) => {
                self.text = full_text.split('\n').map(ToOwned::to_owned).collect();
                T::parse(&self.text, &mut self.tokens);
            }
            Payload::Completion(_, _c, id) => {
                let items = self.definitions.to_completions(&self.tokens);
                let completion_response = CompletionResponse::Array(items);
                let response = match to_value(completion_response) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(err) => Response { id, result: None, error: Some(Value::String(err.to_string())) },
                };
                self.responses.lock().unwrap().insert(id, response);
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
        T::parse(&self.text, &mut self.tokens);
        Some(())
    }
}

#[derive(Debug, PartialEq)]
struct PositionedToken<T: LangStream> {
    from: usize,
    len: usize,
    token_type: u32,
    modifier: u32,
    lang_token: T,
}

impl<T: LangStream> PositionedToken<T> {
    #[inline]
    pub fn refresh_type(&mut self) {
        self.token_type = self.lang_token.type_id();
        self.modifier = self.lang_token.modifier();
    }

    #[inline]
    pub fn semantic_token(&self, delta_line: u32, at_char: usize) -> SemanticToken {
        SemanticToken {
            delta_line,
            length: self.len as u32,
            delta_start: (self.from - at_char) as u32,
            token_type: self.token_type,
            token_modifiers_bitset: self.modifier,
        }
    }
}

#[derive(Default)]
struct Definitions {
    types: Vec<Struct>,
    function: Vec<Func>,
    variables: Vec<Var>,
    keywords: Vec<&'static str>,
}

impl Definitions {
    fn to_completions<T: LangStream>(&self, tokens: &[Vec<PositionedToken<T>>]) -> Vec<CompletionItem> {
        let mut items = self
            .keywords
            .iter()
            .map(|kward| CompletionItem::new_simple((*kward).to_owned(), String::from("Keyword")))
            .collect::<Vec<_>>();

        let mut fn_set = self.function.iter().map(|func| func.name.to_owned()).collect::<HashSet<_>>();
        let mut var_set = self.variables.iter().map(|var| var.name.to_owned()).collect::<HashSet<_>>();
        let mut type_set = self.types.iter().map(|obj_type| obj_type.name.to_owned()).collect::<HashSet<_>>();

        for tok in tokens.iter().flatten() {
            match tok.lang_token.objectify() {
                ObjType::Var(name) => {
                    var_set.insert(name.to_owned());
                }
                ObjType::Fn(name) => {
                    fn_set.insert(name.to_owned());
                }
                ObjType::Struct(name) => {
                    type_set.insert(name.to_owned());
                }
                _ => (),
            }
        }

        for func in fn_set.into_iter() {
            items.push(CompletionItem {
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                insert_text: Some(format!("{}($0)", func)),
                label: func,
                detail: Some("Function".to_owned()),
                ..Default::default()
            });
        }

        for var in var_set.into_iter() {
            items.push(CompletionItem::new_simple(var, "Variable".to_owned()));
        }

        for type_name in type_set.into_iter() {
            items.push(CompletionItem::new_simple(type_name, "Type".to_owned()));
        }

        items
    }
}

enum ObjType<'a> {
    Fn(&'a str),
    Var(&'a str),
    Struct(&'a str),
    None,
}

struct Struct {
    name: String,
}

impl Struct {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Default)]
struct Func {
    name: String,
}

struct Var {
    name: String,
}
