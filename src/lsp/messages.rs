use lsp_types::{
    notification::{Notification, PublishDiagnostics},
    DiagnosticSeverity, PublishDiagnosticsParams,
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
