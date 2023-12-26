use crate::configs::GeneralAction;
use anyhow::Result;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Default)]
pub struct EditorTerminal {
    pub active: bool,
    history: Vec<String>,
    process: Option<(Child, JoinHandle<()>)>,
    path: String,
    prompt: String,
    at_line: usize,
    max_rows: usize,
    out_buffer: Arc<Mutex<Vec<String>>>,
    cmd_buffer: String,
}

impl EditorTerminal {
    pub fn new() -> Self {
        Self { path: build_path(PathBuf::from("./")), ..Default::default() }
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
        frame.render_widget(List::new(self.get_list_widget()).block(Block::default().borders(Borders::TOP)), tmux_area);
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

    pub async fn map(&mut self, general_action: &GeneralAction) -> bool {
        if !self.active {
            return false;
        }
        match general_action {
            GeneralAction::Up => {
                //TODO prev command
            }
            GeneralAction::Down => {
                //TODO next command
            }
            GeneralAction::Char(ch) => {
                self.cmd_buffer.push(*ch);
                self.prompt_to_last_line();
            }
            GeneralAction::BackspaceInput => {
                self.cmd_buffer.pop();
                self.prompt_to_last_line();
            }
            GeneralAction::ToggleTerminal | GeneralAction::FileTreeModeOrCancelInput | GeneralAction::Exit => {
                self.active = false;
                self.prompt_to_last_line();
            }
            GeneralAction::FinishOrSelect => {
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
            self.prompt = format!("[{}] {branch}$ {}", self.path, self.cmd_buffer)
        } else {
            self.prompt = format!("[{}]$ {}", self.path, self.cmd_buffer)
        }
    }

    async fn push_buffer(&mut self) -> Result<()> {
        if self.cmd_buffer == "clear" {
            self.history.push(self.prompt.to_owned());
            self.cmd_buffer.clear();
            self.at_line = self.history.len().checked_sub(1).unwrap_or_default();
            return Ok(());
        }
        let mut inner = Command::new("sh")
            .arg("-c")
            .arg(self.cmd_buffer.as_str())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        self.history.push(self.prompt.to_owned());
        self.cmd_buffer.clear();
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

fn build_path(path: PathBuf) -> String {
    let base = path.canonicalize().unwrap_or_default();
    base.display().to_string()
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
