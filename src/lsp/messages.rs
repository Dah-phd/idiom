use tokio::process::ChildStdin;

#[derive(Debug)]
pub enum LSPMessage {
    Request {
        id: String,
        method: String,
        params: Option<serde_json::Value>,
    },
    Response {
        id: i64,
        result: serde_json::Value,
    },
    ResponseErr {
        id: i64,
        error: serde_json::Value,
    },
    Notification {
        method: String,
        params: Option<serde_json::Value>,
    },
}

impl LSPMessage {
    pub fn parse(lsp_message: &str) -> Option<LSPMessage> {
        if let Some(json_start) = lsp_message.find('{') {
            if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(&lsp_message[json_start..]) {
                if let Some(id) = obj.get_mut("id") {
                    let id = id.take();
                    if let Some(result) = &mut obj.get_mut("result") {
                        return Some(LSPMessage::Response {
                            id: id.as_i64()?,
                            result: result.take(),
                        });
                    }
                    if let Some(error) = obj.get_mut("error") {
                        return Some(LSPMessage::ResponseErr {
                            id: id.as_i64()?,
                            error: error.take(),
                        });
                    }
                    if let Some(method) = obj.get_mut("method") {
                        return Some(LSPMessage::Request {
                            id: id.to_string(),
                            method: method.to_string(),
                            params: obj.get_mut("params").map(|p| p.take()),
                        });
                    }
                }
                if let Some(method) = obj.get("method") {
                    return Some(LSPMessage::Notification {
                        method: method.to_string(),
                        params: obj.get_mut("params").map(|p| p.take()),
                    });
                }
            }
        };
        None
    }
}

#[allow(unused_variables)]
pub async fn done_auto_response(lsp_message: &mut LSPMessage, stdin: &mut ChildStdin) -> bool {
    if let LSPMessage::Request { id, method, params } = lsp_message {
        #[allow(clippy::match_single_binding)]
        match method.as_str() {
            _ => (),
        }
    }
    false
}
