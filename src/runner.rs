use crate::global_state::GlobalState;
use crate::render::layout::Rect;
use crate::render::{pty::PopupApplet, TextField};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct EditorTerminal {
    terminal: Option<PopupApplet>,
    shell: String,
}

impl EditorTerminal {
    pub fn new(shell: String) -> Self {
        Self { shell, ..Default::default() }
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        if let Some(term) = self.terminal.as_mut() {
            term.render(gs);
        }
    }

    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        if let Some(term) = self.terminal.as_mut() {
            term.fast_render(gs);
        }
    }

    pub fn activate(&mut self, rect: Rect) {
        if self.terminal.is_none() {
            let max_rows = rect.height / 2;
            let rect = rect.bot(max_rows);
            if let Ok(term) = PopupApplet::run(&self.shell, rect) {
                self.terminal.replace(term);
            }
        }
    }

    fn kill(&mut self, _gs: &mut GlobalState) {
        self.terminal.take();
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        match key {
            KeyEvent { code: KeyCode::Esc, .. }
            | KeyEvent { code: KeyCode::Char('`' | ' '), modifiers: KeyModifiers::CONTROL, .. } => {
                gs.message("Term: PTY active in background ... (CTRL + d/q) can be used to kill the process!");
                gs.toggle_terminal(self);
            }
            event_key => {
                if let Some(term) = self.terminal.as_mut() {
                    term.key_map(event_key, &mut gs.clipboard, &gs.matcher);
                }
            }
        }
        true
    }

    pub fn paste_passthrough(&mut self, clip: String) {}

    pub fn resize(&mut self, width: u16) {
        todo!()
    }
}
