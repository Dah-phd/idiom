mod generic;
mod js;
mod python;
mod rust;
mod ts;

use crate::lsp::local::{generic::GenericToken, python::PyToken};
use crate::lsp::{messages::Response, Diagnostics, LSPError, LSPResult, Responses};
use crate::utils::force_lock;
use crate::{configs::FileType, lsp::client::Payload, workspace::CursorPosition};
use logos::Span;
use lsp_types::SemanticTokenType;
use lsp_types::{
    notification::{DidChangeTextDocument, DidOpenTextDocument, Notification},
    Range, SemanticToken, SemanticTokens, SemanticTokensRangeResult, SemanticTokensResult,
};
use serde_json::{from_str, to_value, Value};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

/// Trait to be implemented on the lang specific token, allowing parsing and deriving builtins
trait LangStream: Sized {
    fn init_definitions() -> Definitions;
    fn type_id(&self) -> u32;
    fn modifier(&self) -> u32;
    fn to_postioned(self, span: Span) -> PositionedToken<Self> {
        PositionedToken {
            from: span.start,
            to: span.end,
            len: span.len(),
            token_type: self.type_id(),
            modifier: self.modifier(),
            lang_token: self,
        }
    }
    fn parse(defs: &mut Definitions, text: &Vec<String>, tokens: &mut Vec<Vec<PositionedToken<Self>>>);
    fn parse_semantics(text: &Vec<String>, tokens: &mut Vec<Vec<PositionedToken<Self>>>);
}

/// Not fully blowns LSP - but struct processing tokens better, giving basic utils, like semantics, autocomplete, rename
#[derive(Default)]
struct LocalLSP<T: LangStream> {
    definitions: Definitions,
    text: Vec<String>,
    tokens: Vec<Vec<PositionedToken<T>>>,
    responses: Arc<Responses>,
    diagnostics: Arc<Diagnostics>,
}

pub fn start_lsp_handler(
    mut rx: UnboundedReceiver<Payload>,
    file_type: FileType,
    responses: Arc<Responses>,
    diagnostics: Arc<Diagnostics>,
) -> JoinHandle<LSPResult<()>> {
    match file_type {
        FileType::Python => tokio::task::spawn(async move {
            let mut lsp = LocalLSP::<PyToken>::init(responses, diagnostics);
            while let Some(payload) = rx.recv().await {
                lsp.parase_payload(payload)?;
            }
            Ok(())
        }),
        _ => tokio::task::spawn(async move {
            let mut lsp = LocalLSP::<GenericToken>::init(responses, diagnostics);
            while let Some(payload) = rx.recv().await {
                lsp.parase_payload(payload)?;
            }
            Ok(())
        }),
    }
}

impl<T: LangStream> LocalLSP<T> {
    fn init(responses: Arc<Responses>, diagnostics: Arc<Diagnostics>) -> Self {
        Self { definitions: T::init_definitions(), text: Vec::new(), tokens: Vec::new(), diagnostics, responses }
    }

    fn parase_payload(&mut self, payload: Payload) -> LSPResult<()> {
        match payload {
            Payload::Direct(data) => {
                self.direct_parsing(data)?;
            }
            Payload::Tokens(_, id) => {
                let tokens = SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data: self.full_tokens() });
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(error) => Response { id, result: None, error: Some(Value::String(error.to_string())) },
                };
                force_lock(&self.responses).insert(id, response);
            }
            Payload::PartialTokens(_, range, id, ..) => {
                let tokens = SemanticTokensRangeResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: self.partial_tokens(range),
                });
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(error) => Response { id, result: None, error: Some(Value::String(error.to_string())) },
                };
                force_lock(&self.responses).insert(id, response);
            }
            _ => todo!(),
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
        self.text.extend(text.split('\n').map(ToOwned::to_owned));
        T::parse(&mut self.definitions, &self.text, &mut self.tokens);
        Some(())
    }

    fn full_tokens(&self) -> Vec<SemanticToken> {
        let mut tokens = Vec::new();
        let mut last_delta = 0;
        for token_line in self.tokens.iter() {
            let mut at_char = 0;
            for token in token_line.iter().filter(stylable_tokens) {
                tokens.push(SemanticToken {
                    delta_line: std::mem::take(&mut last_delta),
                    length: token.len as u32,
                    delta_start: (token.from - at_char) as u32,
                    token_type: token.token_type,
                    token_modifiers_bitset: token.modifier,
                });
                at_char = token.from;
            }
            last_delta += 1;
        }
        tokens
    }

    fn partial_tokens(&self, range: Range) -> Vec<SemanticToken> {
        let start = CursorPosition::from(range.start);
        let end = CursorPosition::from(range.end);
        let mut tokens = Vec::new();
        let mut last_delta = start.line as u32;
        let mut remaining = end.line - start.line;
        if remaining == 0 {
            let mut at_char = 0;
            for token in self.tokens[start.line].iter().filter(stylable_tokens) {
                if token.from >= start.char && token.from <= end.char {
                    tokens.push(SemanticToken {
                        delta_line: std::mem::take(&mut last_delta),
                        length: (token.to - token.from) as u32,
                        delta_start: (token.from - at_char) as u32,
                        token_type: token.token_type,
                        token_modifiers_bitset: token.modifier,
                    });
                    at_char += token.to;
                }
            }
            return tokens;
        }
        let mut iter = self.tokens[start.line..=end.line].iter();
        match iter.next() {
            Some(token_line) => {
                let mut at_char = 0;
                for token in token_line.iter().filter(stylable_tokens).filter(|t| t.from >= start.char) {
                    tokens.push(SemanticToken {
                        delta_line: std::mem::take(&mut last_delta),
                        length: (token.to - token.from) as u32,
                        delta_start: (token.from - at_char) as u32,
                        token_type: token.token_type,
                        token_modifiers_bitset: token.modifier,
                    });
                    at_char += token.to;
                }
                last_delta += 1;
            }
            None => return tokens,
        }
        remaining -= 1;
        while remaining > 0 {
            match iter.next() {
                Some(token_line) => {
                    let mut at_char = 0;
                    for token in token_line.iter().filter(stylable_tokens) {
                        tokens.push(SemanticToken {
                            delta_line: std::mem::take(&mut last_delta),
                            length: (token.to - token.from) as u32,
                            delta_start: (token.from - at_char) as u32,
                            token_type: token.token_type,
                            token_modifiers_bitset: token.modifier,
                        });
                        at_char += token.to;
                    }
                    last_delta += 1;
                }
                None => return tokens,
            }
            remaining -= 1;
        }
        match iter.next() {
            Some(token_line) => {
                let mut at_char = 0;
                for token in token_line.iter().filter(stylable_tokens).filter(|t| t.from <= end.char) {
                    tokens.push(SemanticToken {
                        delta_line: std::mem::take(&mut last_delta),
                        length: (token.to - token.from) as u32,
                        delta_start: (token.from - at_char) as u32,
                        token_type: token.token_type,
                        token_modifiers_bitset: token.modifier,
                    });
                    at_char += token.to;
                }
                last_delta += 1;
            }
            None => return tokens,
        }
        tokens
    }
}

struct PositionedToken<T> {
    from: usize,
    to: usize,
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
}

pub fn get_local_legend() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::NAMESPACE,      // 0
        SemanticTokenType::TYPE,           // 1
        SemanticTokenType::CLASS,          // 2
        SemanticTokenType::ENUM,           // 3
        SemanticTokenType::INTERFACE,      // 4
        SemanticTokenType::STRUCT,         // 5
        SemanticTokenType::TYPE_PARAMETER, // 6
        SemanticTokenType::PARAMETER,      // 7
        SemanticTokenType::VARIABLE,       // 8
        SemanticTokenType::PROPERTY,       // 9
        SemanticTokenType::FUNCTION,       // 10
        SemanticTokenType::KEYWORD,        // 11
        SemanticTokenType::COMMENT,        // 12
        SemanticTokenType::STRING,         // 13
        SemanticTokenType::NUMBER,         // 14
        SemanticTokenType::DECORATOR,      // 15
    ]
}

fn stylable_tokens<T: LangStream>(token: &&PositionedToken<T>) -> bool {
    token.token_type < 16
}

#[derive(Default)]
struct Definitions {
    structs: Vec<Struct>,
    function: Vec<Func>,
    variables: Vec<Var>,
}

struct Struct {
    name: String,
    parent: usize,
    attribute: Vec<String>,
    methods: Vec<String>,
}

impl Struct {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), parent: 0, methods: vec![], attribute: vec![] }
    }

    const fn parent(mut self, parent_id: usize) -> Self {
        self.parent = parent_id;
        self
    }

    fn attr(mut self, name: impl Into<String>) -> Self {
        self.attribute.push(name.into());
        self
    }

    fn meth(mut self, name: impl Into<String>) -> Self {
        self.methods.push(name.into());
        self
    }
}

#[derive(Default)]
struct Func {
    name: String,
    args: Vec<usize>,
    returns: Option<usize>,
}

struct Var {
    name: String,
    var_type: usize,
}

#[cfg(test)]
mod test {
    use lsp_types::SemanticToken;

    use crate::lsp::local::LangStream;

    use super::{python::PyToken, LocalLSP};
    use std::sync::Arc;

    #[test]
    fn test_with_pytoken() {
        let mut pylsp = LocalLSP::<PyToken>::init(Arc::default(), Arc::default());
        pylsp.text.push(String::from("class WorkingDirectory:"));
        PyToken::parse(&mut pylsp.definitions, &pylsp.text, &mut pylsp.tokens);
        let tokens = pylsp.full_tokens();
        assert_eq!(
            tokens,
            vec![
                SemanticToken { delta_line: 0, delta_start: 0, length: 5, token_type: 11, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 6, length: 16, token_type: 1, token_modifiers_bitset: 0 }
            ]
        );
    }
}
