use anyhow::Result;
use serde::Serialize;
use serde_json::to_string;

#[derive(Serialize)]
pub struct LSPNotification<T>
where
    T: lsp_types::notification::Notification,
    T::Params: serde::Serialize,
{
    jsonrpc: String,
    pub method: &'static str,
    params: T::Params,
}

impl<T> LSPNotification<T>
where
    T: lsp_types::notification::Notification,
    T::Params: serde::Serialize,
{
    pub fn with(params: T::Params) -> Self {
        Self { jsonrpc: String::from("2.0"), method: <T as lsp_types::notification::Notification>::METHOD, params }
    }

    pub fn stringify(&self) -> Result<String> {
        let request_msg = to_string(self)?;
        let ser_req = format!("Content-Length: {}\r\n\r\n{}", request_msg.len(), request_msg);
        Ok(ser_req)
    }
}
