use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize};
use serde::Serialize;
use std::{
    io::{BufReader, Read, Write},
    sync::{Arc, Mutex},
};
use strip_ansi_escapes::strip_str;
use tokio::task::JoinHandle;

use crate::error::{IdiomError, IdiomResult};
use dirs::config_dir;

pub struct Terminal {
    pair: PtyPair,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output_handler: JoinHandle<()>,
    output: Arc<Mutex<Vec<String>>>,
}

impl Terminal {
    pub fn new(shell: &str, width: u16) -> IdiomResult<(Self, Arc<Mutex<String>>)> {
        let system = native_pty_system();
        let pair = system
            .openpty(PtySize { rows: 24, cols: width, ..Default::default() })
            .map_err(|err| IdiomError::any(err))?;
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd("./");
        let child = pair.slave.spawn_command(cmd).map_err(|error| IdiomError::any(error))?;
        let writer = pair.master.take_writer().map_err(|error| IdiomError::any(error))?;
        let reader = pair.master.try_clone_reader().map_err(|error| IdiomError::any(error))?;
        let output: Arc<Mutex<Vec<String>>> = Arc::default();
        let buffer = Arc::clone(&output);
        let prompt: Arc<Mutex<String>> = Arc::default();
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
                                        if data == "\n" {
                                            buffer.lock().unwrap().push(strip_str(&l));
                                            l.clear();
                                        } else {
                                            l.push_str(data);
                                            *prompt_writer.lock().unwrap() = strip_str(&l);
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
        self.writer.write_all(b"\n")?;
        #[cfg(windows)]
        self.writer.write_all(b"\r\n")?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, cols: u16) -> IdiomResult<()> {
        self.pair
            .master
            .resize(PtySize { rows: 24, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|err| IdiomError::io_other(format!("Term Resize Err: {err}")))
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.output_handler.abort();
        let _ = self.child.kill();
    }
}

pub fn overwrite_cfg<T: Default + Serialize>(f: &str) -> IdiomResult<String> {
    let mut path = match config_dir() {
        Some(path) => path,
        None => {
            return Err(IdiomError::io_not_found("Filed to derive config dir!"));
        }
    };
    path.push(f);
    let data = serde_json::to_string_pretty(&T::default())
        .map_err(|err| IdiomError::io_other(format!("Parsing Err: {err}")))?;
    std::fs::write(&path, data)?;
    Ok(path.display().to_string())
}
