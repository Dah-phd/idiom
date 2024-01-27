mod commands;
use crate::configs::{EDITOR_CFG_FILE, KEY_MAP, THEME_FILE};
use crate::global_state::GlobalState;
use anyhow::Result;
use commands::{build_command, load_cfg};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;
use std::path::{PathBuf, MAIN_SEPARATOR};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin};
use tokio::task::JoinHandle;

#[derive(Default)]
pub struct EditorTerminal {
    pub active: bool,
    git_branch: Option<String>,
    idiom_prefix: String,
    logs: Vec<String>,
    at_log: usize,
    cmd_histroy: Vec<String>,
    at_history: usize,
    process: Option<(ChildStdin, Child, JoinHandle<()>)>,
    path: PathBuf,
    prompt: String,
    max_rows: usize,
    out_buffer: Arc<Mutex<Vec<String>>>,
}

impl EditorTerminal {
    pub fn new() -> Self {
        Self {
            git_branch: get_branch(),
            path: PathBuf::from("./").canonicalize().unwrap_or_default(),
            idiom_prefix: String::from("%i"),
            cmd_histroy: vec!["".to_owned()],
            logs: vec![
                "This is not a true terminal but command executor.".to_owned(),
                "It holds only basic functionality but does not support continious processes (not a pty).".to_owned(),
                "Main goal is to have easy acces to git and build tools (such as cargo/pybuilder/tsc).".to_owned(),
                "idiom specific commands are triggered with prefix %i <cmd> use %i help to get more info.".to_owned(),
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
            List::new(self.get_list_widget()).block(Block::default().title("Runner").borders(Borders::TOP)),
            tmux_area,
        );
        screen_areas[0]
    }

    fn get_list_widget(&mut self) -> Vec<ListItem> {
        self.build_prompt();
        let mut list = self
            .logs
            .iter()
            .skip(self.at_log)
            .take(self.max_rows)
            .map(|line| ListItem::new(line.to_owned()))
            .collect::<Vec<ListItem>>();
        list.push(self.prompt.to_owned().into());
        list
    }

    fn prompt_to_last_line(&mut self) {
        if self.logs.len().checked_sub(self.max_rows).unwrap_or_default() > self.at_log {
            self.at_log = (self.logs.len() + 2).checked_sub(self.max_rows).unwrap_or_default();
        }
    }

    async fn kill(&mut self, gs: &mut GlobalState) {
        if let Some((.., mut child, handler)) = self.process.take() {
            match child.try_wait() {
                Ok(Some(..)) => {}
                Ok(None) => {
                    let _ = child.kill().await;
                    self.logs.push("KeyBoard Interrupt!".to_owned());
                }
                Err(err) => {
                    gs.error(format!("Runner process failed: {err}"));
                }
            }
            handler.abort();
        }
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
            KeyEvent { code: KeyCode::PageUp, .. }
            | KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::CONTROL, .. } => {
                self.at_log = self.at_log.checked_sub(1).unwrap_or_default();
            }
            KeyEvent { code: KeyCode::PageDown, .. }
            | KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::CONTROL, .. } => {
                self.at_log = std::cmp::min(self.at_log + 1, self.logs.len());
            }
            KeyEvent { code: KeyCode::Up, .. } => {
                self.at_history = self.at_history.checked_sub(1).unwrap_or_default();
            }
            KeyEvent { code: KeyCode::Down, .. } => {
                self.at_history = std::cmp::min(self.at_history + 1, self.cmd_histroy.len() - 1)
            }
            KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::CONTROL, .. } => {
                self.kill(gs).await;
            }
            KeyEvent { code: KeyCode::Char(ch), .. } => {
                self.cmd_histroy[self.at_history].push(*ch);
                self.prompt_to_last_line();
            }
            KeyEvent { code: KeyCode::Backspace, .. } => {
                self.cmd_histroy[self.at_history].pop();
                self.prompt_to_last_line();
            }
            KeyEvent { code: KeyCode::Enter, .. } => {
                let _ = self.push_command(gs).await;
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
        self.logs.extend(guard.drain(..));
        drop(guard);
        self.prompt_to_last_line();
    }

    fn build_prompt(&mut self) {
        if let Some(branch) = self.git_branch.as_ref() {
            self.prompt = format!("[{}] {branch}$ {}", self.path.display(), self.cmd_histroy[self.at_history])
        } else {
            self.prompt = format!("[{}]$ {}", self.path.display(), self.cmd_histroy[self.at_history])
        }
    }

    async fn push_command(&mut self, gs: &mut GlobalState) -> Result<()> {
        self.git_branch = get_branch();
        let command = self.cmd_histroy[self.at_history].trim().to_owned();
        self.cmd_histroy.push(String::new());
        self.at_history = self.cmd_histroy.len() - 1;
        if let Some((stdin, child, handler)) = self.process.as_mut() {
            match child.try_wait() {
                Ok(None) => {
                    stdin.write_all(command.as_bytes()).await?;
                    stdin.flush().await?;
                    self.logs.push(command.to_owned());
                    return Ok(());
                }
                Err(err) => {
                    gs.error(format!("Runner process failed: {err}"));
                    handler.abort();
                }
                _ => (),
            }
        }
        self.logs.push(self.prompt.to_owned());
        if let Some(arg) = command.strip_prefix(&self.idiom_prefix) {
            return self.idiom_command_handler(arg, gs);
        }
        if command == "clear" {
            self.at_log = self.logs.len();
            return Ok(());
        }
        if let Some(arg) = command.strip_prefix("cd ") {
            self.cd(arg);
            return Ok(());
        }
        self.process.replace(build_command(&command, &self.path, &self.out_buffer)?);
        Ok(())
    }

    fn cd(&mut self, arg: &str) {
        if arg.starts_with("..") {
            for _ in arg.split(MAIN_SEPARATOR) {
                if let Some(parent) = self.path.parent() {
                    self.path = PathBuf::from(parent).canonicalize().unwrap_or_default();
                }
            }
        } else {
            let mut buffer = self.path.to_owned();
            buffer.push(arg);
            if let Ok(path) = buffer.canonicalize() {
                if path.is_dir() {
                    self.path = path;
                }
            }
        }
    }

    pub fn idiom_command_handler(&mut self, arg: &str, gs: &mut GlobalState) -> Result<()> {
        if arg.trim() == "clear" {
            let old = std::mem::replace(self, Self::new());
            self.active = true;
            let _ = old.process.map(|(_, _, process)| process.abort());
        }
        if arg.trim() == "help" {
            self.logs.push("load => load config files, available options:".to_owned());
            self.logs.push("    keymap => open keymap config file.".to_owned());
            self.logs.push("    config => open editor config file.".to_owned());
            self.logs.push("    theme => open theme config file.".to_owned());
            self.logs.push("Example: &i load keymap".to_owned());
        }
        if arg.trim() == "loc" {
            self.process.replace(build_command("git ls-files | xargs wc -l", &self.path, &self.out_buffer)?);
        }
        if let Some(cfg) = arg.trim().strip_prefix("load") {
            if let Some(msg) = match cfg.trim() {
                "keymap" => load_cfg(KEY_MAP, gs),
                "config" => load_cfg(EDITOR_CFG_FILE, gs),
                "theme" => load_cfg(THEME_FILE, gs),
                _ => {
                    self.logs.push("Invalid arg on %i load <cfg>".to_owned());
                    self.logs.push(format!("Bad arg: {}", cfg));
                    self.logs.push("Expected: keymap | config | theme!".to_owned());
                    None
                }
            } {
                self.logs.push(msg);
            }
        }
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
