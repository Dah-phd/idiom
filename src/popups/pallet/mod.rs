mod formatting;
use super::{popup_file_open::OpenFileSelector, Command, CommandResult, Components, InplacePopup, Status};
use crate::{
    configs::{EDITOR_CFG_FILE, KEY_MAP, THEME_FILE, THEME_UI},
    global_state::{GlobalState, IdiomEvent},
    render::{layout::Rect, state::State, TextField},
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use fuzzy_matcher::FuzzyMatcher;

pub struct Pallet {
    commands: Vec<(i64, Command)>,
    pattern: TextField<bool>,
    rect: Option<Rect>,
    state: State,
}

impl InplacePopup for Pallet {
    type R = ();

    fn force_render(&mut self, gs: &mut GlobalState) {
        let mut rect = gs.screen_rect.top(15).vcenter(100);
        let backend = gs.backend();
        rect.bordered();
        self.rect.replace(rect);
        rect.draw_borders(None, None, backend);
        match rect.next_line() {
            Some(line) => self.pattern.widget(line, backend),
            None => return,
        }
        let options = self.commands.iter().map(|cmd| cmd.1.label);
        self.state.render_list(options, rect, backend);
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut super::Components) -> super::Status<Self::R> {
        let Components { gs, ws, tree, .. } = components;

        if self.commands.is_empty() {
            return Status::Dropped;
        }

        if let Some(updated) = self.pattern.map(&key, &mut gs.clipboard) {
            if updated {
                for (score, cmd) in self.commands.iter_mut() {
                    *score = match gs.matcher.fuzzy_match(cmd.label, &self.pattern.text) {
                        Some(new_score) => new_score,
                        None => i64::MAX,
                    };
                }
                self.commands.sort_by(|(score, _), (rhscore, _)| score.cmp(rhscore));
            }
            return Status::Pending;
        }
        match key.code {
            KeyCode::Enter => {
                match self.commands.remove(self.state.selected).1.execute() {
                    CommandResult::Simple(event) => gs.event.push(event),
                    CommandResult::Complex(cb) => cb(ws, tree),
                }
                return Status::Dropped;
            }
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                self.state.prev(self.commands.len());
                self.force_render(gs);
            }
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.state.next(self.commands.len());
                self.force_render(gs);
            }
            _ => (),
        }
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut super::Components) -> super::Status<Self::R> {
        let Components { gs, ws, tree, .. } = components;

        let (row, column) = match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => (row, column),
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.commands.len());
                self.force_render(gs);
                return Status::Pending;
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.commands.len());
                self.force_render(gs);
                return Status::Pending;
            }
            _ => return Status::Pending,
        };
        let relative_row = match self.rect.as_ref().and_then(|rect| rect.relative_position(row, column)) {
            Some(pos) => pos.line,
            None => return Status::Pending,
        };
        if relative_row < 1 {
            return Status::Pending;
        }
        let command_index = self.state.at_line + (relative_row - 1);
        if self.commands.len() <= command_index {
            return Status::Pending;
        }
        match self.commands.remove(command_index).1.execute() {
            CommandResult::Simple(event) => gs.event.push(event),
            CommandResult::Complex(cb) => (cb)(ws, tree),
        };
        Status::Dropped
    }

    fn paste_passthrough(&mut self, clip: String, components: &mut super::Components) -> bool {
        if !self.pattern.paste_passthrough(clip) {
            return false;
        }
        for (score, cmd) in self.commands.iter_mut() {
            *score = match components.gs.matcher.fuzzy_match(cmd.label, &self.pattern.text) {
                Some(new_score) => new_score,
                None => i64::MAX,
            };
        }
        self.commands.sort_by(|(score, _), (rhscore, _)| score.cmp(rhscore));
        true
    }

    fn resize_success(&mut self, gs: &mut GlobalState) -> bool {
        self.force_render(gs);
        true
    }

    fn render(&mut self, _: &mut GlobalState) {}
}

impl Pallet {
    pub fn new(_screen: Rect) -> Box<Self> {
        let commands = [
            Some(Command::pass_event("Open file", IdiomEvent::NewPopup(OpenFileSelector::boxed))),
            Some(Command::pass_event("GitUI", IdiomEvent::EmbededApp(String::from("gitui")))),
            Some(Command::access_edit("UPPERCASE", formatting::uppercase)),
            Some(Command::access_edit("LOWERCASE", formatting::lowercase)),
            Command::cfg_open("Open editor configs", EDITOR_CFG_FILE),
            Command::cfg_open("Open keymap config", KEY_MAP),
            Command::cfg_open("Open theme config", THEME_FILE),
            Command::cfg_open("Open UI theme config", THEME_UI),
            Some(Command::pass_event("Open editor error log", IdiomEvent::OpenLSPErrors)),
        ]
        .into_iter()
        .flatten()
        .map(|cmd| (0, cmd))
        .collect();

        Box::new(Pallet {
            commands,
            pattern: TextField::new(String::new(), Some(true)),
            rect: None,
            state: State::new(),
        })
    }
}

#[cfg(test)]
mod tests;
