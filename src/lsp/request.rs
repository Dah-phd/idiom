use anyhow::Result;
use serde::Serialize;
use serde_json::to_string;

#[derive(Serialize)]
pub struct LSPRequest<T>
where
    T: lsp_types::request::Request,
    T::Params: serde::Serialize,
    T::Result: serde::de::DeserializeOwned,
{
    jsonrpc: String,
    id: usize,
    pub method: &'static str,
    params: T::Params,
}

impl<T> LSPRequest<T>
where
    T: lsp_types::request::Request,
    T::Params: serde::Serialize,
    T::Result: serde::de::DeserializeOwned,
{
    pub fn with(id: usize, params: T::Params) -> Self {
        Self {
            jsonrpc: String::from("2.0"),
            id,
            method: <T as lsp_types::request::Request>::METHOD,
            params,
        }
    }

    pub fn stringify(&self) -> Result<String> {
        let request_msg = to_string(self)?;
        let ser_req = format!("Content-Length: {}\r\n\r\n{}", request_msg.len(), request_msg);
        // std::fs::write("req.json", &ser_req);
        Ok(ser_req)
    }
}
