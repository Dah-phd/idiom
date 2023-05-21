pub mod rust;
pub mod request;
use request::LSPRequest;
use std::collections::HashMap;
use std::future::Future;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

use lsp_types::request::{Initialize, RegisterCapability, Request};
use lsp_types::{InitializeParams, Url, WorkspaceFolder, RegistrationParams, Registration};

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
    pub async fn start(server: Command) -> std::io::Result<Self> {
        let init_request: LSPRequest<Initialize> = LSPRequest::with(
            1,
            InitializeParams {
                process_id: Some(std::process::id()),
                ..Default::default()
            },
        );
        let mut lsp = Self::new(server).await?;
        let result = lsp.send(init_request.stringify(), init_request.method).await;
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
            requests: HashMap::default(),
            inner,
            handler,
            stdin,
        })
    }

    async fn send(&mut self, lsp_request: String, method: &'static str) -> std::io::Result<()> {
        self.requests.insert(self.counter, (method, lsp_request.to_owned()));
        self.counter += 1;
        let _ = self.stdin.write(lsp_request.as_bytes()).await?;
        self.stdin.flush().await
    }

    pub fn register(&mut self, registrations: Vec<Registration>) {
        self.counter += 1;
        let request:LSPRequest<RegisterCapability> = LSPRequest::with(
            self.counter,
            RegistrationParams {
                registrations
            }
        );
        self.send(request.stringify(), request.method);
    }

    fn dash_nine(&mut self) {
        self.handler.abort();
        self.inner.kill();
    }
}
