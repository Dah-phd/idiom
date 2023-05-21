use serde::Serialize;
use serde_json::to_string;
use lsp_types::request::{Request};

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

    pub fn stringify(&self) -> String {
        if let Ok(request) = to_string(self) {
            format!("Content-Length: {}\r\n\r\n{}", request.len(), request)
        } else {
            "".to_owned()
        }
    }
}