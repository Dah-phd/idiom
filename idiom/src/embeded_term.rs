use crate::global_state::GlobalState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use idiom_ui::backend::Backend;
use idiom_ui::layout::{Line, Rect, BORDERS};
use idiom_ui::pty::{Message, PtyShell, OVERLAY_INFO};

#[derive(Default)]
pub struct EditorTerminal {
    terminal: Option<PtyShell>,
    shell: Option<String>,
    border: Option<Line>,
}

impl EditorTerminal {
    pub fn new(shell: Option<String>) -> Self {
        Self { shell, ..Default::default() }
    }

    pub fn set_shell(&mut self, shell: Option<String>) {
        self.shell = shell;
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        if let Some(border) = self.border.clone() {
            border.fill(BORDERS.horizontal_top, gs.backend());
        }
        if let Some(term) = self.terminal.as_mut() {
            gs.message(OVERLAY_INFO);
            gs.message(OVERLAY_INFO);
            term.render(gs.backend());
        }
    }

    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        if let Some(term) = self.terminal.as_mut() {
            term.fast_render(gs.backend());
            if term.is_finished() {
                self.terminal = None;
                gs.toggle_terminal(self);
            }
        }
    }

    pub fn activate(&mut self, rect: Rect) {
        if self.terminal.is_none() {
            let max_rows = rect.height / 2;
            let mut rect = rect.bot(max_rows);
            self.border = rect.next_line();
            if let Ok(term) =
                self.shell.as_ref().map(|shell| PtyShell::run(shell, rect)).unwrap_or(PtyShell::default_cmd(rect))
            {
                self.terminal.replace(term);
            }
        }
    }

    fn kill(&mut self, _gs: &mut GlobalState) {
        self.terminal.take();
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        match key {
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => {
                self.kill(gs);
                gs.success("Term: Process killed!");
                gs.toggle_terminal(self);
            }
            KeyEvent { code: KeyCode::Char('`' | ' '), modifiers: KeyModifiers::CONTROL, .. } => {
                gs.message("Term: PTY active in background ... (CTRL + q) can be used to kill the process!");
                gs.toggle_terminal(self);
                gs.backend().hide_cursor();
            }
            event_key => {
                if let Some(term) = self.terminal.as_mut() {
                    if let Err(error) = term.map_key(event_key, gs.backend()) {
                        gs.error(error);
                        self.terminal.take();
                        gs.toggle_terminal(self);
                    };
                }
            }
        }
        true
    }

    pub fn map_mouse(&mut self, event: MouseEvent, gs: &mut GlobalState) {
        if let Some(term) = self.terminal.as_mut() {
            match term.map_mouse(event, gs.backend()) {
                Message::Copied(clip) => {
                    gs.clipboard.push(clip);
                    gs.success("Select from embeded copied!");
                }
                Message::Skipped(MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)) => {
                    gs.toggle_terminal(self);
                    gs.backend().hide_cursor();
                }
                _ => {}
            };
        };
    }

    pub fn paste_passthrough(&mut self, clip: String) {
        if let Some(term) = self.terminal.as_mut() {
            _ = term.paste(clip);
        }
    }

    pub fn resize(&mut self, rect: Rect) {
        if let Some(pty) = self.terminal.as_mut() {
            let max_rows = rect.height / 2;
            let mut rect = rect.bot(max_rows);
            self.border = rect.next_line();
            _ = pty.resize(rect);
        }
    }
}
