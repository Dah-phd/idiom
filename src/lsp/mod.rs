pub mod rust;
pub mod request;
use request::LSPRequest;
use serde_json::to_string;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};
use anyhow::{Result, anyhow};

use lsp_types::lsp_request;
use lsp_types::{InitializeParams, Url, WorkspaceFolder, RegistrationParams, Registration};

use crate::messages::FileType;

#[allow(clippy::upper_case_acronyms)]
pub struct LSP {
    que: Arc<Mutex<Vec<String>>>,
    counter: usize,
    requests: HashMap<usize, (&'static str, String)>,
    inner: Child,
    handler: JoinHandle<()>,
    stdin: ChildStdin,
}

impl LSP {
    pub async fn from(file_type: &FileType) -> Result<Self> {
        match file_type {
            FileType::Rust => Self::new(rust::start_lsp()).await,
            _ => Err(anyhow!("Not supported LSP!"))
        }
    }

    pub async fn new(mut server: Command) -> Result<Self> {
        let mut inner = server
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        let mut stdin = inner.stdin.take().unwrap();

        let pwd_uri = format!("file://{}", std::env::current_dir()?.as_os_str().to_str().unwrap());
        let request: LSPRequest<lsp_request!("initialize")> = LSPRequest::with(
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
        let _ = stdin.write(to_string(&request)?.as_bytes()).await?;
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
            requests: HashMap::default(),
            inner,
            handler,
            stdin,
        })
    }

    fn process_request(&mut self) {
        let result = self.que.lock().unwrap().pop();
        if let Some(data) = result {

        }
    }

    async fn send(&mut self, lsp_request: String, method: &'static str) -> Result<()> {
        self.requests.insert(self.counter, (method, lsp_request.to_owned()));
        self.counter += 1;
        let _ = self.stdin.write(lsp_request.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub async fn register(&mut self, registrations: Vec<Registration>) -> Result<()> {
        self.counter += 1;
        let request:LSPRequest<lsp_request!("client/registerCapability")> = LSPRequest::with(
            self.counter,
            RegistrationParams {
                registrations
            }
        );

        self.send(to_string(&request)?, request.method).await
    }

    fn dash_nine(&mut self) {
        self.handler.abort();
        self.inner.kill();
    }
}
