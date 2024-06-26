use lsp_types::{
    notification::{Notification, PublishDiagnostics},
    request::GotoDeclarationResponse,
    CompletionItem, CompletionResponse, DiagnosticSeverity, GotoDefinitionResponse, Hover, Location,
    PublishDiagnosticsParams, SemanticTokensRangeResult, SemanticTokensResult, SignatureHelp, WorkspaceEdit,
};
use serde_json::{from_value, Value};
use std::path::PathBuf;

use crate::{
    lsp::{LSPError, LSPResult},
    syntax::DiagnosticLine,
};

use super::lsp_stream::StdErrMessage;

pub enum LSPMessage {
    Request(Request),
    Response(Response),
    Notification(GeneralNotification),
    Diagnostic(PathBuf, Diagnostic),
    Unknown(Value),
    Error(String),
}

impl LSPMessage {
    pub fn unwrap(self) -> LSPResult<Value> {
        // gets value within if data is know at check time
        // errors on response error
        match self {
            Self::Unknown(raw) => Some(raw),
            Self::Notification(notification) => notification.params,
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

    pub fn parse(mut obj: Value) -> LSPMessage {
        if let Some(id) = obj.get_mut("id") {
            let id = id.take();
            if let Some(result) = &mut obj.get_mut("result") {
                return LSPMessage::Response(Response {
                    id: id.as_i64().unwrap(),
                    result: Some(result.take()),
                    error: None,
                });
            }
            if let Some(error) = obj.get_mut("error") {
                return LSPMessage::Response(Response {
                    id: id.as_i64().unwrap(),
                    result: None,
                    error: Some(error.take()),
                });
            }
            if let Some(method) = obj.get_mut("method") {
                return LSPMessage::Request(Request {
                    id: id.to_string(),
                    method: method.to_string(),
                    params: obj.get_mut("params").map(|p| p.take()),
                });
            }
        }
        if let Some(method) = obj.get("method") {
            if method == PublishDiagnostics::METHOD {
                let params = obj.get_mut("params").map(|p| p.take()).unwrap();
                let diagnostics = from_value::<PublishDiagnosticsParams>(params).unwrap();
                return LSPMessage::Diagnostic(diagnostics.uri.as_str()[7..].into(), Diagnostic::new(diagnostics));
            }
            return LSPMessage::Notification(GeneralNotification {
                method: method.to_string(),
                params: obj.get_mut("params").map(|p| p.take()),
            });
        };
        LSPMessage::Unknown(obj)
    }
}

impl From<Value> for LSPMessage {
    fn from(obj: Value) -> Self {
        Self::parse(obj)
    }
}

impl From<StdErrMessage> for LSPMessage {
    fn from(err: StdErrMessage) -> Self {
        Self::Error(err.0)
    }
}

#[derive(Debug)]
pub struct Request {
    pub id: String,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug)]
pub struct Response {
    pub id: i64,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

#[derive(Debug)]
pub struct GeneralNotification {
    pub method: String,
    pub params: Option<Value>,
}

/// Stores Diagnostics and metadata - to be used in editor to gain access to diagnostic params objects.
/// updated flag is used to ensure only updated diagnostics are sent.
pub struct Diagnostic {
    pub errors: usize,
    pub warnings: usize,
    pub lines: Option<Vec<(usize, DiagnosticLine)>>,
}

impl Diagnostic {
    fn new(params: PublishDiagnosticsParams) -> Self {
        let mut diagnostic_lines: Vec<(usize, DiagnosticLine)> = Vec::new();
        let mut errors = 0;
        let mut warnings = 0;
        for d in params.diagnostics {
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
    Completion(i64, String, usize),
    Hover(i64),
    SignatureHelp(i64),
    References(i64),
    Renames(i64),
    Tokens(i64),
    TokensPartial {
        id: i64,
        max_lines: usize,
    },
    #[allow(dead_code)]
    Definition(i64),
    Declaration(i64),
}

impl LSPResponseType {
    pub fn id(&self) -> &i64 {
        match self {
            Self::Completion(id, ..) => id,
            Self::Hover(id) => id,
            Self::SignatureHelp(id) => id,
            Self::References(id) => id,
            Self::Renames(id) => id,
            Self::Tokens(id) => id,
            Self::TokensPartial { id, .. } => id,
            Self::Definition(id) => id,
            Self::Declaration(id) => id,
        }
    }

    pub fn parse(&self, value: Option<Value>) -> Option<LSPResponse> {
        Some(match self {
            Self::Completion(.., line, idx) => match from_value::<CompletionResponse>(value?).ok()? {
                CompletionResponse::Array(arr) => LSPResponse::Completion(arr, line.to_owned(), *idx),
                CompletionResponse::List(ls) => LSPResponse::Completion(ls.items, line.to_owned(), *idx),
            },
            Self::Hover(..) => LSPResponse::Hover(from_value(value?).ok()?),
            Self::SignatureHelp(..) => LSPResponse::SignatureHelp(from_value(value?).ok()?),
            Self::References(..) => LSPResponse::References(from_value(value?).ok()?),
            Self::Renames(..) => LSPResponse::Renames(from_value(value?).ok()?),
            Self::Tokens(..) => LSPResponse::Tokens(from_value(value?).ok()?),
            Self::TokensPartial { max_lines, .. } => {
                LSPResponse::TokensPartial { result: from_value(value?).ok()?, max_lines: *max_lines }
            }
            Self::Definition(..) => LSPResponse::Definition(from_value(value?).ok()?),
            Self::Declaration(..) => LSPResponse::Declaration(from_value(value?).ok()?),
        })
    }
}

pub enum LSPResponse {
    Completion(Vec<CompletionItem>, String, usize),
    Hover(Hover),
    SignatureHelp(SignatureHelp),
    References(Option<Vec<Location>>),
    Renames(WorkspaceEdit),
    Tokens(SemanticTokensResult),
    TokensPartial { result: SemanticTokensRangeResult, max_lines: usize },
    Definition(GotoDefinitionResponse),
    Declaration(GotoDeclarationResponse),
}
