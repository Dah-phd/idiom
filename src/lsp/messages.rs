use std::path::PathBuf;

use lsp_types::{
    notification::{Notification, PublishDiagnostics},
    DiagnosticSeverity, PublishDiagnosticsParams,
};
use serde_json::{from_str, from_value, Value};
use tokio::process::ChildStdin;

#[derive(Debug)]
pub enum LSPMessage {
    Request(Request),
    Response(Response),
    Notification(GeneralNotification),
    Diagnostic(PathBuf, Diagnostic),
}

impl LSPMessage {
    pub fn parse(lsp_message: &str) -> Option<LSPMessage> {
        if let Some(json_start) = lsp_message.find('{') {
            if let Ok(mut obj) = from_str::<serde_json::Value>(&lsp_message[json_start..]) {
                if let Some(id) = obj.get_mut("id") {
                    let id = id.take();
                    if let Some(result) = &mut obj.get_mut("result") {
                        return Some(LSPMessage::Response(Response {
                            id: id.as_i64()?,
                            error: None,
                            result: Some(result.take()),
                        }));
                    }
                    if let Some(error) = obj.get_mut("error") {
                        return Some(LSPMessage::Response(Response {
                            id: id.as_i64()?,
                            result: None,
                            error: Some(error.take()),
                        }));
                    }
                    if let Some(method) = obj.get_mut("method") {
                        return Some(LSPMessage::Request(Request {
                            id: id.to_string(),
                            method: method.to_string(),
                            params: obj.get_mut("params").map(|p| p.take()),
                        }));
                    }
                }
                if let Some(method) = obj.get("method") {
                    if method == PublishDiagnostics::METHOD {
                        let params = obj.get_mut("params").map(|p| p.take())?;
                        let diagnostics = from_value::<PublishDiagnosticsParams>(params).ok()?;
                        return Some(LSPMessage::Diagnostic(
                            diagnostics.uri.as_str()[7..].into(),
                            Diagnostic::new(diagnostics),
                        ));
                    }
                    return Some(LSPMessage::Notification(GeneralNotification {
                        method: method.to_string(),
                        params: obj.get_mut("params").map(|p| p.take()),
                    }));
                }
            }
        };
        None
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

#[allow(unused_variables)]
pub async fn done_auto_response(lsp_message: &mut Request, stdin: &mut ChildStdin) -> bool {
    #[allow(clippy::match_single_binding)]
    match lsp_message.method.as_str() {
        _ => (),
    }
    false
}
