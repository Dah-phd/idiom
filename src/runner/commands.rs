use portable_pty::PtyPair;
use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
use serde::Serialize;
use std::path::PathBuf;
use std::{
    io::{BufReader, Read, Write},
    sync::{Arc, Mutex},
};
use strip_ansi_escapes::strip_str;
use tokio::task::JoinHandle;

#[cfg(unix)]
const SHELL: &str = "bash";
#[cfg(windows)]
const SHELL: &str = "cmd";

use crate::error::{IdiomError, IdiomResult};
use crate::global_state::IdiomEvent;
use crate::{configs::CONFIG_FOLDER, global_state::GlobalState, utils::force_lock};
use dirs::config_dir;

pub struct Terminal {
    pair: PtyPair,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output_handler: JoinHandle<()>,
    output: Arc<Mutex<Vec<String>>>,
}

impl Terminal {
    pub fn new(width: u16) -> IdiomResult<(Self, Arc<Mutex<String>>)> {
        let system = native_pty_system();
        let pair = system
            .openpty(PtySize { rows: 24, cols: width, ..Default::default() })
            .map_err(|err| IdiomError::any(err.to_string()))?;
        let mut cmd = CommandBuilder::new(SHELL);
        cmd.cwd("./");
        let child = pair.slave.spawn_command(cmd).map_err(|err| IdiomError::any(err.to_string()))?;
        let writer = pair.master.take_writer().map_err(|err| IdiomError::any(err.to_string()))?;
        let reader = pair.master.try_clone_reader().map_err(|err| IdiomError::any(err.to_string()))?;
        let output = Arc::default();
        let buffer = Arc::clone(&output);
        let prompt = Arc::default();
        let prompt_writer = Arc::clone(&prompt);
        Ok((
            Self {
                pair,
                output,
                child,
                writer,
                output_handler: tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut l = String::new();
                    for result in BufReader::new(reader).bytes() {
                        match result {
                            Ok(byte) => {
                                bytes.push(byte);
                                match std::str::from_utf8(&bytes) {
                                    Ok(data) => {
                                        l.push_str(data);
                                        let cleaned = strip_str(&l);
                                        if cleaned.ends_with('\n') {
                                            force_lock(&buffer).push(cleaned);
                                            l.clear();
                                        } else {
                                            *force_lock(&prompt_writer) = cleaned;
                                        }
                                        bytes.clear();
                                    }
                                    Err(..) => continue,
                                }
                            }
                            Err(_) => return,
                        }
                    }
                }),
            },
            prompt,
        ))
    }

    pub fn kill(mut self) -> IdiomResult<()> {
        self.output_handler.abort();
        self.child.kill()?;
        Ok(())
    }

    pub fn pull_logs(&mut self) -> Option<Vec<String>> {
        if let Ok(mut guard) = self.output.try_lock() {
            if !guard.is_empty() {
                return Some(guard.drain(..).collect());
            }
        }
        None
    }

    pub fn is_running(&mut self) -> bool {
        self.output_handler.is_finished() || self.child.try_wait().is_ok()
    }

    pub fn push_command(&mut self, cmd: String) -> std::io::Result<()> {
        self.writer.write_all(cmd.as_bytes())?;
        #[cfg(unix)]
        self.writer.write_all(&[b'\n'])?;
        #[cfg(windows)]
        self.writer.write_all(&[b'\r', b'\n'])?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, cols: u16) -> IdiomResult<()> {
        self.pair
            .master
            .resize(PtySize { rows: 24, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|err| IdiomError::io_err(format!("Term Resize Err: {err}")))
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.output_handler.abort();
        let _ = self.child.kill();
    }
}

pub fn load_file(f: &str, gs: &mut GlobalState) -> Option<String> {
    let path = PathBuf::from(f);
    match path.canonicalize() {
        Ok(path) => {
            gs.event.push(IdiomEvent::OpenAtLine(path, 0));
            None
        }
        Err(err) => Some(err.to_string()),
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
    gs.event.push(IdiomEvent::OpenAtLine(path, 0));
    None
}

pub fn overwrite_cfg<T: Default + Serialize>(f: &str) -> IdiomResult<String> {
    let mut path = match config_dir() {
        Some(path) => path,
        None => {
            return Err(IdiomError::io_err("Filed to derive config dir!"));
        }
    };
    path.push(f);
    let data =
        serde_json::to_string_pretty(&T::default()).map_err(|err| IdiomError::io_err(format!("Parsing Err: {err}")))?;
    std::fs::write(&path, data)?;
    Ok(path.display().to_string())
}
