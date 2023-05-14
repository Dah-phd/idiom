pub mod rust;
use tokio::io::AsyncWriteExt;
use tokio::process::{Command, Child};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, BytesCodec};

use lsp_types::{InitializeParams, WorkspaceFolder, Url};
use lsp_types::request::{Request, Initialize};
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
    method: &'static str,
    params: T::Params,
}

impl<T> LSPRequest<T>
where
    T: lsp_types::request::Request,
    T::Params: serde::Serialize,
    T::Result: serde::de::DeserializeOwned,
{
    fn with(id: usize, params: T::Params) -> Self {
        Self {
            jsonrpc: String::from("2.0"),
            id: 0,
            method: <T as lsp_types::request::Request>::METHOD,
            params,
        }
    }

    fn stringify(&self) -> String {
        if let Ok(request) = to_string(self) {
            format!("Content-Length: {}\r\n\r\n{}", request.len(), request)
        } else {
            "".to_owned()
        }
    }
}

pub struct LSP <T> 
where
    T:Request
{   
    que: Vec<String>,
    counter: usize,
    requests: Vec<LSPRequest<T>>
}

impl<T> LSP<T> 
where
    T:Request
{
    pub fn start(server: Command) {
        let init_request: LSPRequest<Initialize> = LSPRequest::with(
            0,
            InitializeParams {
                process_id: Some(std::process::id()),
                ..Default::default()
            },
        );
        let lsp = Self::new(server);
        lsp.send(init_request.stringify());
    }

    fn new(server: Command) -> Self {
        Self { counter: 1, requests:vec![], que: vec![] }
    }

    fn send(&self, lsp_request: String) {}
}



pub async fn init(mut cmd: Command) -> (Child, JoinHandle<()>) {
    let mut child = cmd.spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let request: LSPRequest<Initialize> = LSPRequest::with(
        0,
        InitializeParams {
            process_id: Some(std::process::id()),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::parse("file:///home/dah/Documents/4_rust/idiom").unwrap(),
                name: "root".to_owned(),
            }]),
            ..Default::default()
        },
    );
    stdin.write(request.stringify().as_bytes()).await;
    stdin.flush().await;
    let stderr = child.stderr.take().unwrap();
    let stderr = FramedRead::new(stderr, BytesCodec::new());
    let stdout = child.stdout.take().unwrap();
    let stdout = FramedRead::new(stdout, BytesCodec::new());
    let mut stream = stdout.chain(stderr);
    let handler = tokio::task::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            let string = String::from_utf8_lossy(&msg);
            println!("{:?}", string);
        }
    });
    (child, handler)
}