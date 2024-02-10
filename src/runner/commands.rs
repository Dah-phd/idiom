use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use portable_pty::PtyPair;
use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
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

use crate::{
    configs::CONFIG_FOLDER,
    global_state::{GlobalState, WorkspaceEvent},
    utils::into_guard,
};
use dirs::config_dir;

pub struct Terminal {
    pair: PtyPair,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output_handler: JoinHandle<()>,
    output: Arc<Mutex<Vec<String>>>,
}

impl Terminal {
    pub fn new() -> Result<(Self, Arc<Mutex<String>>)> {
        let system = native_pty_system();
        let pair = system.openpty(PtySize { rows: 24, cols: 80, ..Default::default() })?;
        let mut cmd = CommandBuilder::new(SHELL);
        cmd.cwd("./");
        let child = pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;
        let reader = pair.master.try_clone_reader()?;
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
                                            into_guard(&buffer).push(cleaned);
                                            l.clear();
                                        } else {
                                            *into_guard(&prompt_writer) = cleaned;
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

    pub fn map(&mut self, key: &KeyEvent) -> Result<()> {
        let msg = match key.code {
            KeyCode::Char(ch) => {
                if key.modifiers == KeyModifiers::CONTROL && ch == 'l' {
                    vec![27, 91, 50, 74]
                } else {
                    vec![ch as u8]
                }
            }
            #[cfg(unix)]
            KeyCode::Enter => vec![b'\n'],
            #[cfg(windows)]
            KeyCode::Enter => vec![b'\r', b'\n'],
            KeyCode::Backspace => vec![8],
            KeyCode::Left => vec![27, 91, 68],
            KeyCode::Right => vec![27, 91, 67],
            KeyCode::Up => vec![27, 91, 65],
            KeyCode::Down => vec![27, 91, 66],
            KeyCode::Tab => vec![9],
            KeyCode::Home => vec![27, 91, 72],
            KeyCode::End => vec![27, 91, 70],
            KeyCode::PageUp => vec![27, 91, 53, 126],
            KeyCode::PageDown => vec![27, 91, 54, 126],
            KeyCode::BackTab => vec![27, 91, 90],
            KeyCode::Delete => vec![27, 91, 51, 126],
            KeyCode::Insert => vec![27, 91, 50, 126],
            KeyCode::Esc => vec![27],
            _ => vec![],
        };
        self.writer.write_all(&msg)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn kill(mut self) -> Result<()> {
        self.output_handler.abort();
        self.child.kill()?;
        Ok(())
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

    pub fn resize(&mut self, cols: u16) -> Result<()> {
        self.pair.master.resize(PtySize { rows: 24, cols, pixel_width: 0, pixel_height: 0 })
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.output_handler.abort();
        let _ = self.child.kill();
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
