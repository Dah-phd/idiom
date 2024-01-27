use anyhow::Result;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::process::{Child, ChildStdin, Command};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

use crate::{
    configs::CONFIG_FOLDER,
    global_state::{GlobalState, WorkspaceEvent},
};
use dirs::config_dir;

pub fn build_command(
    command: &str,
    current_dir: &Path,
    buffer: &Arc<Mutex<Vec<String>>>,
) -> Result<(ChildStdin, Child, JoinHandle<()>)> {
    let mut inner = Command::new("sh")
        .current_dir(current_dir)
        .arg("-c")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .arg(command)
        .spawn()?;
    let out_handler = Arc::clone(buffer);
    let stderr = FramedRead::new(inner.stderr.take().unwrap(), LinesCodec::new());
    let stdout = FramedRead::new(inner.stdout.take().unwrap(), LinesCodec::new());
    let stdin = inner.stdin.take().unwrap();
    let mut stream = stdout.chain(stderr);
    let join_handler = tokio::spawn(async move {
        while let Some(Ok(line)) = stream.next().await {
            match out_handler.lock() {
                Ok(mut guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            }
            .push(line)
        }
    });
    Ok((stdin, inner, join_handler))
}

pub fn load_cfg(f: &str, gs: &mut GlobalState) -> Option<String> {
    let mut path = match config_dir() {
        Some(path) => path,
        None => {
            return Some("Unable to resolve config dir".to_owned());
        }
    };
    path.push(CONFIG_FOLDER);
    path.push(f);
    gs.workspace.push_back(WorkspaceEvent::Open(path, 0));
    None
}
