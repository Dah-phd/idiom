use anyhow::{anyhow, Result};
use lsp_types::{
    request::Initialize, ClientCapabilities, HoverClientCapabilities, InitializeParams, MarkupKind,
    ReferenceClientCapabilities, SignatureHelpClientCapabilities, TextDocumentClientCapabilities,
    TextDocumentSyncClientCapabilities, Url, WorkspaceClientCapabilities, WorkspaceFolder,
};
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
    pub id: usize,
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
        Ok(ser_req)
    }

    pub fn init_request() -> Result<LSPRequest<Initialize>> {
        let pwd_uri = format!(
            "file:///{}",
            std::env::current_dir()?
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("pwd conversion err"))?
        );
        let uri = Url::parse(&pwd_uri)?;
        Ok(LSPRequest::with(
            0,
            InitializeParams {
                workspace_folders: Some(vec![WorkspaceFolder {
                    uri,
                    name: "root".to_owned(),
                }]),
                capabilities: ClientCapabilities {
                    workspace: Some(WorkspaceClientCapabilities { ..Default::default() }),
                    text_document: Some(TextDocumentClientCapabilities {
                        synchronization: Some(TextDocumentSyncClientCapabilities {
                            will_save: Some(true),
                            did_save: Some(true),
                            ..Default::default()
                        }),
                        hover: Some(HoverClientCapabilities {
                            content_format: Some(vec![MarkupKind::PlainText]),
                            ..Default::default()
                        }),
                        references: Some(ReferenceClientCapabilities::default()),
                        signature_help: Some(SignatureHelpClientCapabilities {
                            context_support: Some(true),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
    }
}
