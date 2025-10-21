use lsp_types::{
    notification::{Notification, PublishDiagnostics},
    request::GotoDeclarationResponse,
    CompletionItem, CompletionResponse, DiagnosticSeverity, GotoDefinitionResponse, Hover, Location,
    PublishDiagnosticsParams, SemanticTokensRangeResult, SemanticTokensResult, SignatureHelp, Uri, WorkspaceEdit,
};
use serde_json::{from_value, Result as SerdeResult, Value};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Display,
    path::PathBuf,
};

use crate::{
    lsp::{LSPError, LSPResult},
    syntax::{tokens::reforamt_delta_tokens, DiagnosticLine},
    workspace::CursorPosition,
};

use super::lsp_stream::StdErrMessage;

pub enum LSPMessage {
    Request(Request),
    Response(Response),
    Diagnostic(Uri, Diagnostic),
    Unknown(Value),
    Error(String),
}

impl LSPMessage {
    pub fn unwrap(self) -> LSPResult<Value> {
        // gets value within if data is know at check time
        // errors on response error
        match self {
            Self::Unknown(raw) => Some(raw),
            Self::Response(resp) => {
                if resp.result.is_some() {
                    resp.result
                } else {
                    return Err(LSPError::ResponseError(format!("{:?}", resp.error)));
                }
            }
            Self::Request(request) => request.params,
            _ => None,
        }
        .ok_or(LSPError::internal("Called unwrap on LSPMessage type not supporting the operand!"))
    }
}

impl From<Value> for LSPMessage {
    fn from(mut obj: Value) -> Self {
        if let Some(raw_id) = obj.get("id").cloned() {
            if let Some(id) = raw_id.as_i64() {
                if let Some(result) = &mut obj.get_mut("result") {
                    return LSPMessage::Response(Response { id, result: Some(result.take()), error: None });
                }
                if let Some(error) = obj.get_mut("error") {
                    return LSPMessage::Response(Response { id, result: None, error: Some(error.take()) });
                }
            }
            if let Some(method) = obj.get_mut("method") {
                return LSPMessage::Request(Request {
                    _id: raw_id.to_string(),
                    _method: method.to_string(),
                    params: obj.get_mut("params").map(|p| p.take()),
                });
            }
        }
        if let Some(method) = obj.get("method") {
            if method == PublishDiagnostics::METHOD {
                if let Some(PublishDiagnosticsParams { uri, diagnostics, .. }) = obj
                    .get_mut("params")
                    .map(Value::take)
                    .and_then(|params| from_value::<PublishDiagnosticsParams>(params).ok())
                {
                    return LSPMessage::Diagnostic(uri, Diagnostic::new(diagnostics));
                }
            }
        };
        LSPMessage::Unknown(obj)
    }
}

impl From<StdErrMessage> for LSPMessage {
    fn from(err: StdErrMessage) -> Self {
        Self::Error(err.0)
    }
}

#[derive(Debug)]
pub struct Request {
    pub _id: String,
    pub _method: String,
    pub params: Option<Value>,
}

#[derive(Debug)]
pub struct Response {
    pub id: i64,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

pub type EditorDiagnostics = Vec<(usize, DiagnosticLine)>;
pub type TreeDiagnostics = Vec<(PathBuf, DiagnosticType)>;

#[derive(Default)]
pub struct DiagnosticHandle {
    meta: HashMap<PathBuf, DiagnosticType>,
    diffs: Vec<(PathBuf, DiagnosticType)>,
    files: HashMap<Uri, crate::lsp::Diagnostic>,
}

impl DiagnosticHandle {
    pub fn collect(&mut self, uri: &Uri) -> (Option<EditorDiagnostics>, Option<TreeDiagnostics>) {
        (
            self.files.get_mut(uri).and_then(|d| d.lines.take()),
            if self.meta.is_empty() { None } else { Some(std::mem::take(&mut self.diffs)) },
        )
    }

    pub fn insert(&mut self, k: Uri, v: crate::lsp::Diagnostic) {
        if v.errors != 0 {
            self.push_meta(k.as_str(), DiagnosticType::Err);
        } else if v.warnings != 0 {
            self.push_meta(k.as_str(), DiagnosticType::Warn);
        } else {
            self.push_meta(k.as_str(), DiagnosticType::None);
        }
        self.files.insert(k, v);
    }

    #[inline]
    fn push_meta(&mut self, uri_text: &str, diagnostic_type: DiagnosticType) {
        if let Some(path) = uri_text.get(7..).map(PathBuf::from).and_then(|p| p.canonicalize().ok()) {
            match self.meta.entry(path.clone()) {
                Entry::Occupied(mut entry) => {
                    if entry.insert(diagnostic_type) == diagnostic_type {
                        return;
                    }
                    self.diffs.push((path, diagnostic_type));
                }
                Entry::Vacant(entry) => {
                    entry.insert(diagnostic_type);
                    self.diffs.push((path, diagnostic_type));
                }
            }
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum DiagnosticType {
    Err,
    Warn,
    None,
}

/// Stores Diagnostics and metadata - to be used in editor to gain access to diagnostic params objects.
/// updated flag is used to ensure only updated diagnostics are sent.
pub struct Diagnostic {
    pub errors: usize,
    pub warnings: usize,
    pub lines: Option<Vec<(usize, DiagnosticLine)>>,
}

impl Diagnostic {
    fn new(diagnostics: Vec<lsp_types::Diagnostic>) -> Self {
        let mut diagnostic_lines: Vec<(usize, DiagnosticLine)> = Vec::new();
        let mut errors = 0;
        let mut warnings = 0;
        for d in diagnostics {
            match d.severity {
                Some(DiagnosticSeverity::ERROR) => errors += 1,
                Some(DiagnosticSeverity::WARNING) => warnings += 1,
                _ => (),
            }
            let line_idx = d.range.start.line as usize;
            if let Some((_, line)) = diagnostic_lines.iter_mut().find(|(idx, _)| idx == &line_idx) {
                line.append(d);
            } else {
                diagnostic_lines.push((line_idx, d.into()));
            }
        }
        Self { errors, warnings, lines: Some(diagnostic_lines) }
    }
}

#[derive(Debug)]
pub enum LSPResponseType {
    Completion(String, CursorPosition),
    Hover,
    SignatureHelp,
    References,
    Renames,
    Tokens,
    TokensPartial { max_lines: usize },
    Definition,
    Declaration,
}

impl LSPResponseType {
    pub fn parse(&self, value: Value) -> SerdeResult<LSPResponse> {
        Ok(match self {
            Self::Completion(.., line, cursor) => match from_value::<CompletionResponse>(value)? {
                CompletionResponse::Array(arr) => LSPResponse::Completion(arr, line.to_owned(), *cursor),
                CompletionResponse::List(ls) => LSPResponse::Completion(ls.items, line.to_owned(), *cursor),
            },
            Self::Hover => LSPResponse::Hover(from_value(value)?),
            Self::SignatureHelp => LSPResponse::SignatureHelp(from_value(value)?),
            Self::References => LSPResponse::References(from_value(value)?),
            Self::Renames => LSPResponse::Renames(from_value(value)?),
            Self::Tokens => {
                let mut tokens = from_value(value)?;
                match &mut tokens {
                    SemanticTokensResult::Tokens(tokens) => reforamt_delta_tokens(&mut tokens.data),
                    SemanticTokensResult::Partial(tokens) => reforamt_delta_tokens(&mut tokens.data),
                };
                LSPResponse::Tokens(tokens)
            }
            Self::TokensPartial { max_lines, .. } => {
                let mut result = from_value(value)?;
                match &mut result {
                    SemanticTokensRangeResult::Tokens(tokens) => reforamt_delta_tokens(&mut tokens.data),
                    SemanticTokensRangeResult::Partial(tokens) => reforamt_delta_tokens(&mut tokens.data),
                }
                LSPResponse::TokensPartial { result, max_lines: *max_lines }
            }
            Self::Definition => LSPResponse::Definition(from_value(value)?),
            Self::Declaration => LSPResponse::Declaration(from_value(value)?),
        })
    }
}

pub enum LSPResponse {
    Completion(Vec<CompletionItem>, String, CursorPosition),
    Hover(Hover),
    SignatureHelp(SignatureHelp),
    References(Option<Vec<Location>>),
    Renames(WorkspaceEdit),
    Tokens(SemanticTokensResult),
    TokensPartial { result: SemanticTokensRangeResult, max_lines: usize },
    Definition(GotoDefinitionResponse),
    Declaration(GotoDeclarationResponse),
    Error(String),
}

impl Display for LSPResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LSPResponseType::Completion(..) => f.write_str("Completion"),
            LSPResponseType::Declaration => f.write_str("Declaration"),
            LSPResponseType::Definition => f.write_str("Definition"),
            LSPResponseType::Hover => f.write_str("Hover"),
            LSPResponseType::Renames => f.write_str("Renames"),
            LSPResponseType::SignatureHelp => f.write_str("SignatureHelp"),
            LSPResponseType::Tokens => f.write_str("Tokens"),
            LSPResponseType::TokensPartial { .. } => f.write_str("TokensPartial"),
            LSPResponseType::References => f.write_str("References"),
        }
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::{Diagnostic, DiagnosticSeverity};

    #[test]
    fn diagnostic_parse() {
        let diags = vec![
            Diagnostic { severity: Some(DiagnosticSeverity::HINT), ..Default::default() },
            Diagnostic { severity: Some(DiagnosticSeverity::ERROR), ..Default::default() },
            Diagnostic { severity: Some(DiagnosticSeverity::WARNING), ..Default::default() },
            Diagnostic { ..Default::default() },
            Diagnostic { severity: Some(DiagnosticSeverity::ERROR), ..Default::default() },
        ];

        let dd = super::Diagnostic::new(diags);
        assert_eq!(dd.errors, 2);
        assert_eq!(dd.warnings, 1);
        let diag_types = dd.lines.unwrap().first().unwrap().1.iter().map(|d| d.severity).collect::<Vec<_>>();
        assert_eq!(
            diag_types,
            [
                DiagnosticSeverity::ERROR,
                DiagnosticSeverity::ERROR,
                DiagnosticSeverity::WARNING,
                DiagnosticSeverity::HINT,
                DiagnosticSeverity::INFORMATION,
            ]
        )
    }
}
