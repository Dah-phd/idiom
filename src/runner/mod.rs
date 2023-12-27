mod commands;
use crate::global_state::GlobalState;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;
use std::path::{PathBuf, MAIN_SEPARATOR};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Default)]
pub struct EditorTerminal {
    active: bool,
    idiom_prefix: String,
    history: Vec<String>,
    cmd_histroy: Vec<String>,
    at_history: usize,
    process: Option<(Child, JoinHandle<()>)>,
    path: PathBuf,
    prompt: String,
    at_line: usize,
    max_rows: usize,
    out_buffer: Arc<Mutex<Vec<String>>>,
    cmd_buffer: String,
}

impl EditorTerminal {
    pub fn new() -> Self {
        Self {
            path: PathBuf::from("./").canonicalize().unwrap_or_default(),
            idiom_prefix: String::from("%i"),
            history: vec![
                "This is not a true terminal but command executor.".to_owned(),
                "It holds only basic functionality but does not support continious processes (not a pty).".to_owned(),
                "Main goal is to have easy acces to git and build tools (such as cargo/pybuilder/tsc).".to_owned(),
            ],
            ..Default::default()
        }
    }

    pub fn render_with_remainder(&mut self, frame: &mut Frame, screen: Rect) -> Rect {
        if !self.active {
            return screen;
        }
        self.poll_results();
        let screen_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Min(2)])
            .split(screen);
        let tmux_area = screen_areas[1];
        self.max_rows = tmux_area.height as usize;
        frame.render_widget(
            List::new(self.get_list_widget()).block(Block::default().title("Terminal").borders(Borders::TOP)),
            tmux_area,
        );
        screen_areas[0]
    }

    fn get_list_widget(&mut self) -> Vec<ListItem> {
        self.build_prompt();
        let mut list = self
            .history
            .iter()
            .skip(self.at_line)
            .take(self.max_rows)
            .map(|line| ListItem::new(line.to_owned()))
            .collect::<Vec<ListItem>>();
        list.push(self.prompt.to_owned().into());
        list
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    fn prompt_to_last_line(&mut self) {
        self.at_line = (self.history.len() + 2).checked_sub(self.max_rows).unwrap_or_default();
    }

    pub async fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        if !self.active {
            return false;
        }
        match key {
            KeyEvent { code: KeyCode::Esc, .. }
            | KeyEvent { code: KeyCode::Char('d' | 'D' | 'q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => {
                self.active = false;
                self.prompt_to_last_line();
            }
            KeyEvent { code: KeyCode::Up, .. } => {
                //TODO prev command
            }
            KeyEvent { code: KeyCode::Down, .. } => {
                //TODO next command
            }
            KeyEvent { code: KeyCode::PageUp, .. } => {}
            KeyEvent { code: KeyCode::PageDown, .. } => {}
            KeyEvent { code: KeyCode::Char(ch), .. } => {
                self.cmd_buffer.push(*ch);
                self.prompt_to_last_line();
            }
            KeyEvent { code: KeyCode::Backspace, .. } => {
                self.cmd_buffer.pop();
                self.prompt_to_last_line();
            }
            KeyEvent { code: KeyCode::Enter, .. } => {
                let _ = self.push_buffer().await;
                self.prompt_to_last_line();
            }
            _ => (),
        }
        true
    }

    fn poll_results(&mut self) {
        let mut guard = match self.out_buffer.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if guard.is_empty() {
            return;
        }
        self.history.extend(guard.drain(..));
        drop(guard);
        self.prompt_to_last_line();
    }

    fn build_prompt(&mut self) {
        if let Some(branch) = get_branch() {
            self.prompt = format!("[{}] {branch}$ {}", self.path.display(), self.cmd_buffer)
        } else {
            self.prompt = format!("[{}]$ {}", self.path.display(), self.cmd_buffer)
        }
    }

    async fn push_buffer(&mut self) -> Result<()> {
        self.history.push(self.prompt.to_owned());
        let command = std::mem::take(&mut self.cmd_buffer);
        if let Some(arg) = command.strip_prefix(&self.idiom_prefix) {
            if arg.trim() == "clear" {
                let mut new = Self::new();
                new.active = true;
                *self = new;
            }
            self.history.push(format!("IDIOM CMD {arg}"));
            return Ok(());
        }
        if command == "clear" {
            self.at_line = self.history.len().checked_sub(1).unwrap_or_default();
            return Ok(());
        }
        if let Some(arg) = command.strip_prefix("cd ") {
            if arg.starts_with("..") {
                for _ in arg.split(MAIN_SEPARATOR) {
                    if let Some(parent) = self.path.parent() {
                        self.path = PathBuf::from(parent).canonicalize().unwrap_or_default();
                    }
                }
            } else if arg.starts_with('/') {
                if let Ok(path) = PathBuf::from(arg).canonicalize() {
                    if path.is_dir() {
                        self.path = path;
                    }
                }
            }
            return Ok(());
        }
        let mut inner = Command::new("sh")
            .current_dir(&self.path)
            .arg("-c")
            .arg(&command)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let out_handler = Arc::clone(&self.out_buffer);
        let stderr = FramedRead::new(inner.stderr.take().unwrap(), BytesCodec::new());
        let stdout = FramedRead::new(inner.stdout.take().unwrap(), BytesCodec::new());
        let mut stream = stdout.chain(stderr);
        let join_handler = tokio::spawn(async move {
            while let Some(Ok(bytes)) = stream.next().await {
                let out = String::from_utf8_lossy(&bytes);
                match out_handler.lock() {
                    Ok(mut guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                }
                .extend(out.lines().map(|s| s.to_owned()))
            }
        });
        self.process.replace((inner, join_handler));
        Ok(())
    }
}

fn get_branch() -> Option<String> {
    let child = std::process::Command::new("sh")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("-c")
        .arg("git branch")
        .spawn()
        .ok()?;
    let output = child.wait_with_output().ok()?;
    let branches = String::from_utf8(output.stdout).ok()?;
    for line in branches.lines() {
        if let Some(branch) = line.trim().strip_prefix('*') {
            return Some(format!("({})", branch.trim()));
        }
    }
    None
}
