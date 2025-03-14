mod autocomplete;
mod commands;
mod components;

use crate::configs::{EditorConfigs, KeyMap, EDITOR_CFG_FILE, KEY_MAP, THEME_FILE};
use crate::error::IdiomResult;
use crate::global_state::GlobalState;
use crate::render::layout::BORDERS;
use crate::render::TextField;
use crate::runner::commands::load_file;
use autocomplete::try_autocomplete;
use commands::{load_cfg, overwrite_cfg, Terminal};
use components::CmdHistory;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::{Arc, Mutex};

const IDIOM_PREFIX: &str = "%i";

#[derive(Default)]
pub struct EditorTerminal {
    cmd_history: CmdHistory,
    cmd: TextField<()>,
    width: u16,
    logs: Vec<String>,
    at_log: usize,
    terminal: Option<Terminal>,
    prompt: Option<Arc<Mutex<String>>>,
    max_rows: usize,
    shell: String,
}

impl EditorTerminal {
    pub fn new(shell: String, width: u16) -> Self {
        Self { shell, width, ..Default::default() }
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        let max_rows = gs.editor_area.height / 2;
        let area = gs.editor_area.bot(max_rows);
        self.max_rows = max_rows as usize;
        self.poll_results();
        let mut logs = self.logs.iter().skip(self.at_log).take(self.max_rows);
        let mut lines = area.into_iter();
        if let Some(line) = lines.next() {
            line.fill(BORDERS.horizontal_top, &mut gs.writer);
        }
        for line in &mut lines {
            match logs.next() {
                Some(log) => line.render(log, &mut gs.writer),
                None => {
                    let prompt = self
                        .prompt
                        .as_ref()
                        .map(|p| p.lock().unwrap().to_owned())
                        .unwrap_or(String::from("[Dead terminal]"));
                    let mut buider = line.unsafe_builder(&mut gs.writer);
                    buider.push(&prompt);
                    self.cmd.insert_formatted_text(buider);
                    break;
                }
            }
        }
        for line in lines {
            line.render_empty(&mut gs.writer);
        }
    }

    pub fn activate(&mut self) {
        match self.terminal.as_mut() {
            Some(terminal) => {
                if terminal.is_running() {
                    return;
                }
                if let Ok((terminal, prompt)) = Terminal::new(&self.shell, self.width) {
                    self.terminal.replace(terminal).map(|t| t.kill());
                    self.prompt.replace(prompt);
                }
            }
            None => {
                if let Ok((terminal, prompt)) = Terminal::new(&self.shell, self.width) {
                    self.terminal.replace(terminal);
                    self.prompt.replace(prompt);
                }
            }
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
            | KeyEvent { code: KeyCode::Char('`'), modifiers: KeyModifiers::CONTROL, .. } => {
                gs.message("Term: PTY active in background ... (CTRL + d/q) can be used to kill the process!");
                gs.toggle_terminal(self);
            }
            KeyEvent { code: KeyCode::Char('d' | 'D' | 'q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => {
                self.terminal.take().map(|t| t.kill());
                self.prompt.take();
                self.at_log = self.logs.len();
                gs.success("Term: Process killed!");
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
                if let Some(text) = self.cmd_history.get_prev() {
                    self.cmd.text_set(text);
                }
            }
            KeyEvent { code: KeyCode::Down, .. } => match self.cmd_history.get_next() {
                Some(text) => self.cmd.text_set(text),
                None => self.cmd.text_set(String::new()),
            },
            KeyEvent { code: KeyCode::Tab, .. } => {
                if let Some(text) = self.cmd.text_get_token_at_cursor().and_then(try_autocomplete) {
                    self.cmd.text_replace_token(&text);
                };
            }
            KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::CONTROL, .. } => {
                self.kill(gs);
                self.at_log = self.logs.len();
                self.logs.push("SIGKILL!".to_owned());
                if let Ok((terminal, prompt)) = Terminal::new(&self.shell, self.width) {
                    self.terminal.replace(terminal).map(|t| t.kill());
                    self.prompt.replace(prompt);
                }
            }
            KeyEvent { code: KeyCode::Enter, .. } => {
                let cmd = self.cmd.text_take();
                self.cmd_history.push(&cmd);
                if let Some(args) = cmd.strip_prefix(IDIOM_PREFIX) {
                    let _ = self.idiom_command_handler(args, gs);
                } else if cmd.trim() == "clear" {
                    self.at_log = self.logs.len();
                } else if let Some(t) = self.terminal.as_mut() {
                    let _ = t.push_command(cmd);
                }
            }
            _ => {
                self.cmd.map(key, &mut gs.clipboard);
                self.go_to_last_log();
            }
        }
        true
    }

    pub fn paste_passthrough(&mut self, clip: String) {
        self.cmd.paste_passthrough(clip);
        self.go_to_last_log();
    }

    fn poll_results(&mut self) {
        if let Some(logs) = self.terminal.as_mut().and_then(|t| t.pull_logs()) {
            self.logs.extend(logs);
            self.go_to_last_log();
        }
    }

    pub fn resize(&mut self, width: u16) {
        if let Some(terminal) = self.terminal.as_mut() {
            let _ = terminal.resize(width);
        }
    }

    fn go_to_last_log(&mut self) {
        let logs_with_prompt = self.logs.len() + 2;
        if self.max_rows + self.at_log < logs_with_prompt {
            self.at_log = logs_with_prompt.saturating_sub(self.max_rows);
        }
    }

    pub fn idiom_command_handler(&mut self, arg: &str, gs: &mut GlobalState) -> IdiomResult<()> {
        if arg.trim() == "help" {
            self.logs.push("load => load config files, available options:".to_owned());
            self.logs.push("    keymap => open keymap config file.".to_owned());
            self.logs.push("    config => open editor config file.".to_owned());
            self.logs.push("    theme => open theme config file.".to_owned());
            self.logs.push("    ${file_path} => loads path into editor.".to_owned());
            self.logs.push("Example: &i load keymap".to_owned());
            self.logs.push("".to_owned());
            self.logs.push("default => returns config file to default".to_owned());
            self.logs.push("    possible files keymap, config".to_owned());
            self.logs.push("Example: &i default keymap".to_owned());
        }
        if arg.trim() == "loc" {
            if let Some(terminal) = self.terminal.as_mut() {
                terminal.push_command(String::from("git ls-files | xargs wc -l"))?;
            }
        }
        if let Some(cfg) = arg.trim().strip_prefix("load") {
            if let Some(msg) = match cfg.trim() {
                "keymap" => load_cfg(KEY_MAP, gs),
                "config" => load_cfg(EDITOR_CFG_FILE, gs),
                "theme" => load_cfg(THEME_FILE, gs),
                any => load_file(any, gs),
            } {
                self.logs.push(msg);
            } else {
                gs.toggle_terminal(self);
            }
        }
        if let Some(cfg) = arg.trim().strip_prefix("default") {
            match match cfg.trim() {
                "keymap" => overwrite_cfg::<KeyMap>(KEY_MAP),
                "config" => overwrite_cfg::<EditorConfigs>(EDITOR_CFG_FILE),
                "theme" => overwrite_cfg::<crate::configs::Theme>(THEME_FILE),
                _ => return Ok(()),
            } {
                Ok(msg) => gs.success(msg),
                Err(err) => gs.error(err.to_string()),
            };
        }
        Ok(())
    }
}
