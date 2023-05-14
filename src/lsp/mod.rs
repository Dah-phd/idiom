pub mod rust;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

use lsp_types::request::{Initialize, Request};
use lsp_types::{InitializeParams, Url, WorkspaceFolder};
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

pub struct LSP<T>
where
    T: Request,
{
    que: Arc<Mutex<Vec<String>>>,
    counter: usize,
    requests: Vec<LSPRequest<T>>,
    inner: Child,
    handler: JoinHandle<()>,
    stdin: ChildStdin,
}

impl<T> LSP<T>
where
    T: Request,
{
    pub async fn start(server: Command) -> std::io::Result<Self> {
        let init_request: LSPRequest<Initialize> = LSPRequest::with(
            0,
            InitializeParams {
                process_id: Some(std::process::id()),
                ..Default::default()
            },
        );
        let mut lsp = Self::new(server).await?;
        let result = lsp.send(init_request.stringify()).await;
        if let Err(_) = result {
            lsp.dash_nine();
            result?;
        };
        Ok(lsp)
    }

    async fn new(mut server: Command) -> std::io::Result<Self> {
        // TODO improve error handling!!! && test
        let mut inner = server
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        let mut stdin = inner.stdin.take().unwrap();

        let pwd_uri = format!("file://{}", std::env::current_dir()?.as_os_str().to_str().unwrap());
        let request: LSPRequest<Initialize> = LSPRequest::with(
            0,
            InitializeParams {
                process_id: Some(std::process::id()),
                workspace_folders: Some(vec![WorkspaceFolder {
                    uri: Url::parse(&pwd_uri).unwrap(),
                    name: "root".to_owned(),
                }]),
                ..Default::default()
            },
        );
        let _ = stdin.write(request.stringify().as_bytes()).await?;
        stdin.flush().await?;
        let stderr = FramedRead::new(inner.stderr.take().unwrap(), BytesCodec::new());
        let stdout = FramedRead::new(inner.stdout.take().unwrap(), BytesCodec::new());
        let mut stream = stdout.chain(stderr);
        let que = Arc::new(Mutex::new(vec![]));
        let que_for_handler = Arc::clone(&que);
        let handler = tokio::task::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let string = String::from_utf8_lossy(&msg);
                que_for_handler.lock().unwrap().push(string.into());
            }
        });
        Ok(Self {
            que,
            counter: 1,
            requests: vec![],
            inner,
            handler,
            stdin,
        })
    }

    async fn send(&mut self, lsp_request: String) -> std::io::Result<()> {
        let _ = self.stdin.write(lsp_request.as_bytes()).await?;
        self.stdin.flush().await
    }

    fn dash_nine(&mut self) {
        self.handler.abort();
        self.inner.kill();
    }
}
