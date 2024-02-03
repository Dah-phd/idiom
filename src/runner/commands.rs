use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

#[cfg(unix)]
const SHELL: &str = "bash";
#[cfg(windows)]
const SHELL: &str = "cmd";

use crate::{
    configs::CONFIG_FOLDER,
    global_state::{GlobalState, WorkspaceEvent},
    utils::into_guard,
};
use dirs::config_dir;

pub struct Terminal {
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output_handler: JoinHandle<()>,
    output: Arc<Mutex<Vec<String>>>,
}

impl Terminal {
    pub fn new(path: &Path) -> Result<Self> {
        let system = native_pty_system();
        let pair = system.openpty(PtySize { rows: 24, cols: 80, ..Default::default() })?;
        let mut cmd = CommandBuilder::new(SHELL);
        cmd.cwd(path);
        let child = pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;
        let reader = pair.master.try_clone_reader()?;
        let output: Arc<Mutex<Vec<String>>> = Arc::default();
        let buffer = Arc::clone(&output);
        Ok(Self {
            output,
            child,
            writer,
            output_handler: tokio::spawn(async move {
                let mut reader = BufReader::new(reader);
                let mut line_buffer = String::new();
                while let Ok(read_test) = reader.read_line(&mut line_buffer) {
                    into_guard(&buffer).push(std::mem::take(&mut line_buffer));
                    if read_test == 0 {
                        return;
                    }
                }
            }),
        })
    }

    pub fn kill(mut self) -> Result<()> {
        self.output_handler.abort();
        self.child.kill()?;
        Ok(())
    }

    fn ensure_dead(&mut self) {
        self.output_handler.abort();
        let _ = self.child.kill();
    }

    pub fn pull_logs(&mut self) -> Option<Vec<String>> {
        if let Ok(mut guard) = self.output.try_lock() {
            return Some(guard.drain(..).collect());
        }
        None
    }

    pub fn is_running(&mut self) -> bool {
        self.output_handler.is_finished() || self.child.try_wait().is_ok()
    }

    pub fn push_command(&mut self, cmd: &str) -> std::io::Result<()> {
        writeln!(self.writer, "{}", cmd)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.ensure_dead();
    }
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
    gs.workspace.push(WorkspaceEvent::Open(path, 0));
    None
}
