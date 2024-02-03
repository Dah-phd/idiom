mod commands;
use crate::configs::{EDITOR_CFG_FILE, KEY_MAP, THEME_FILE};
use crate::global_state::GlobalState;
use anyhow::Result;
use commands::{load_cfg, Terminal};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};
use ratatui::Frame;

#[derive(Default)]
pub struct EditorTerminal {
    pub active: bool,
    idiom_prefix: String,
    logs: Vec<String>,
    at_log: usize,
    cmd_histroy: Vec<String>,
    at_history: usize,
    terminal: Option<Terminal>,
    max_rows: usize,
}

impl EditorTerminal {
    pub fn new() -> Self {
        Self {
            idiom_prefix: String::from("%i"),
            cmd_histroy: vec!["".to_owned()],
            logs: Vec::default(),
            terminal: Terminal::new().ok(),
            ..Default::default()
        }
    }

    pub fn render(&mut self, frame: &mut Frame, screen: Rect) {
        if !self.active {
            return;
        }
        self.poll_results();
        let screen_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Min(2)])
            .split(screen);
        let tmux_area = screen_areas[1];
        self.max_rows = tmux_area.height as usize;
        frame.render_widget(Clear, tmux_area);
        frame.render_widget(
            List::new(self.get_list_items()).block(Block::default().title("Runner").borders(Borders::TOP)),
            tmux_area,
        );
    }

    pub fn activate(&mut self) {
        match self.terminal.as_mut() {
            Some(terminal) => {
                if !terminal.is_running() {
                    if let Ok(terminal) = Terminal::new() {
                        self.terminal.replace(terminal).map(|t| t.kill());
                    }
                }
            }
            None => {
                if let Ok(terminal) = Terminal::new() {
                    self.terminal.replace(terminal);
                }
            }
        }
        self.active = true;
    }

    pub fn get_list_items(&self) -> Vec<ListItem<'static>> {
        let mut list = self
            .logs
            .iter()
            .skip(self.at_log)
            .take(self.max_rows)
            .map(|line| ListItem::new(line.to_owned()))
            .collect::<Vec<ListItem<'_>>>();
        list.push(ListItem::new(format!("runner >>> {}", self.cmd_histroy[self.at_history])));
        list
    }

    fn prompt_to_last_line(&mut self) {
        if self.logs.len().saturating_sub(self.max_rows) > self.at_log {
            self.at_log = (self.logs.len() + 2).saturating_sub(self.max_rows);
        }
    }

    fn kill(&mut self, _gs: &mut GlobalState) {
        if let Some(terminal) = self.terminal.take() {
            let _ = terminal.kill();
        }
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        match key {
            KeyEvent { code: KeyCode::Esc, .. }
            | KeyEvent { code: KeyCode::Char('d' | 'D' | 'q' | 'Q' | '`'), modifiers: KeyModifiers::CONTROL, .. } => {
                self.prompt_to_last_line();
                gs.toggle_terminal(self);
            }
            KeyEvent { code: KeyCode::PageUp, .. }
            | KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::CONTROL, .. } => {
                self.at_log = self.at_log.saturating_sub(1);
            }
            KeyEvent { code: KeyCode::PageDown, .. }
            | KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::CONTROL, .. } => {
                self.at_log = std::cmp::min(self.at_log + 1, self.logs.len());
            }
            KeyEvent { code: KeyCode::Up, .. } => {
                self.at_history = self.at_history.saturating_sub(1);
            }
            KeyEvent { code: KeyCode::Down, .. } => {
                self.at_history = std::cmp::min(self.at_history + 1, self.cmd_histroy.len() - 1)
            }
            KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::CONTROL, .. } => {
                self.kill(gs);
                if let Ok(terminal) = Terminal::new() {
                    self.terminal.replace(terminal);
                }
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
                let _ = self.push_command(gs);
                self.prompt_to_last_line();
            }
            _ => (),
        }
        true
    }

    fn poll_results(&mut self) {
        if let Some(logs) = self.terminal.as_mut().and_then(|t| t.pull_logs()) {
            self.logs.extend(logs);
            self.prompt_to_last_line();
        }
    }

    fn push_command(&mut self, gs: &mut GlobalState) -> Result<()> {
        let cmd = self.cmd_histroy[self.at_history].trim().to_owned();
        if let Some(arg) = cmd.strip_prefix(&self.idiom_prefix) {
            return self.idiom_command_handler(arg, gs);
        }
        self.cmd_histroy.push(String::new());
        self.at_history = self.cmd_histroy.len() - 1;
        if let Some(terminal) = self.terminal.as_mut() {
            assert!(terminal.is_running());
            terminal.push_command(&cmd).unwrap();
        }
        Ok(())
    }

    pub fn idiom_command_handler(&mut self, arg: &str, gs: &mut GlobalState) -> Result<()> {
        if arg.trim() == "clear" {
            if let Some(terminal) = self.terminal.take() {
                terminal.kill()?;
            }
            self.terminal.replace(Terminal::new()?);
        }
        if arg.trim() == "help" {
            self.logs.push("load => load config files, available options:".to_owned());
            self.logs.push("    keymap => open keymap config file.".to_owned());
            self.logs.push("    config => open editor config file.".to_owned());
            self.logs.push("    theme => open theme config file.".to_owned());
            self.logs.push("Example: &i load keymap".to_owned());
        }
        if arg.trim() == "loc" {
            if let Some(terminal) = self.terminal.as_mut() {
                terminal.push_command("git ls-files | xargs wc -l")?;
            }
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
