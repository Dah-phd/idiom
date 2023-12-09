use super::LSPClient;

use anyhow::{anyhow, Result};
use lsp_types::{
    notification::{Notification, PublishDiagnostics},
    DiagnosticSeverity, PublishDiagnosticsParams,
};
use serde_json::{from_value, Value};
use std::path::PathBuf;

#[derive(Debug)]
pub enum LSPMessage {
    Request(Request),
    Response(Response),
    Notification(GeneralNotification),
    Diagnostic(PathBuf, Diagnostic),
    Unknown(Value),
}

impl LSPMessage {
    pub fn unwrap(self) -> Result<Value> {
        // gets value within if data is know at check time
        // errors on response error
        match self {
            Self::Unknown(raw) => Some(raw),
            Self::Notification(notification) => notification.params,
            Self::Response(resp) => {
                if resp.result.is_some() {
                    resp.result
                } else {
                    return Err(anyhow!("Rsponse err: {:?}", resp.error));
                }
            }
            Self::Request(request) => request.params,
            _ => None,
        }
        .ok_or(anyhow!("Unexpected type!"))
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

#[derive(Debug)]
pub struct Diagnostic {
    pub updated: bool,
    pub errors: usize,
    pub warnings: usize,
    pub params: PublishDiagnosticsParams,
}

impl Diagnostic {
    fn new(params: PublishDiagnosticsParams) -> Self {
        let mut errors = 0;
        let mut warnings = 0;
        for diagnostic in &params.diagnostics {
            match diagnostic.severity {
                Some(DiagnosticSeverity::ERROR) => errors += 1,
                _ => warnings += 1,
            }
        }
        Self { updated: true, errors, warnings, params }
    }

    pub fn take(&mut self) -> Option<PublishDiagnosticsParams> {
        if !self.updated {
            return None;
        }
        self.updated = false;
        Some(self.params.clone())
    }
}

#[allow(unused_variables, dead_code)]
pub fn done_auto_response(lsp_message: &mut Request, client: &mut LSPClient) -> bool {
    #[allow(clippy::match_single_binding)]
    match lsp_message.method.as_str() {
        _ => (),
    }
    false
}
