pub mod request;
pub mod rust;
use crate::messages::FileType;
use lsp_types::request::Initialize;
use request::LSPRequest;

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

use lsp_types::lsp_request;
use lsp_types::{InitializeParams, Registration, RegistrationParams, Url, WorkspaceFolder};

#[allow(clippy::upper_case_acronyms)]
pub struct LSP {
    pub que: Arc<Mutex<Vec<String>>>,
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
            _ => Err(anyhow!("Not supported LSP!")),
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
        let stderr = FramedRead::new(inner.stderr.take().unwrap(), BytesCodec::new());
        let stdout = FramedRead::new(inner.stdout.take().unwrap(), BytesCodec::new());
        let mut stream = stdout.chain(stderr);
        let que = Arc::new(Mutex::new(vec![]));
        let que_for_handler = Arc::clone(&que);
        let handler = tokio::task::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let string: String = String::from_utf8_lossy(&msg).into();
                if let Some(json_start) = string.find('{') {
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&string[json_start..]) {
                        std::fs::write("debug.log", "Works!");
                        // println!("{:?}", obj)
                    } else {
                        std::fs::write("debug.log", "Not work!");
                    }
                };
                // std::fs::write("resp.json", &string);
                que_for_handler.lock().unwrap().push(string);
            }
        });
        let ser_req = request.stringify()?;
        let _ = stdin.write(ser_req.as_bytes()).await?;
        stdin.flush().await?;
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
        if let Some(data) = result {}
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
        let request: LSPRequest<lsp_request!("client/registerCapability")> =
            LSPRequest::with(self.counter, RegistrationParams { registrations });

        let ser_req = request.stringify()?;
        self.send(ser_req, request.method).await
    }

    async fn dash_nine(&mut self) -> Result<()> {
        self.handler.abort();
        self.inner.kill().await?;
        Ok(())
    }
}
