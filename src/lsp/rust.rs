use super::LSPRequest;
use lsp_types::{InitializeParams, Url};
use lsp_types::{request::*, WorkspaceFolder};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, BytesCodec};

pub fn start_lsp() -> Command {
    let params: InitializeParams = InitializeParams {
        process_id: Some(std::process::id()),
        ..Default::default()
    };
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg("/home/dah/Downloads/extension/server/rust-analyzer");
    cmd
}
