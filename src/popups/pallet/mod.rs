mod change_state;
mod formatting;
use super::{popup_file_open::OpenFileSelector, Command, CommandResult, Components, Popup, Status};
use crate::{
    configs::{EDITOR_CFG_FILE, KEY_MAP, THEME_FILE, THEME_UI},
    embeded_term::EditorTerminal,
    ext_tui::{text_field::TextField, State},
    global_state::{GlobalState, IdiomEvent},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use fuzzy_matcher::FuzzyMatcher;
use idiom_tui::{layout::Rect, Position};

pub struct Pallet {
    commands: Vec<(i64, Command)>,
    pattern: TextField<bool>,
    state: State,
}

impl Popup for Pallet {
    fn force_render(&mut self, gs: &mut GlobalState) {
        let mut rect = Self::get_rect(gs);
        let backend = gs.backend();
        rect.draw_borders(None, None, backend);
        match rect.next_line() {
            Some(line) => self.pattern.widget(line, backend),
            None => return,
        }
        let options = self.commands.iter().map(|cmd| cmd.1.label);
        self.state.render_list(options, rect, backend);
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut super::Components) -> Status {
        let Components { gs, ws, tree, term } = components;

        if self.commands.is_empty() {
            return Status::Finished;
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
            self.force_render(gs);
            return Status::Pending;
        }
        match key.code {
            KeyCode::Enter => {
                match self.commands.remove(self.state.selected).1.execute() {
                    CommandResult::Simple(event) => gs.event.push(event),
                    CommandResult::BigCB(cb) => cb(gs, ws, tree, term),
                }
                return Status::Finished;
            }
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                self.state.prev(self.commands.len());
            }
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.state.next(self.commands.len());
            }
            _ => (),
        }
        self.force_render(gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut super::Components) -> Status {
        let Components { gs, ws, tree, term } = components;

        match event {
            MouseEvent { kind: MouseEventKind::Moved, column, row, .. } => {
                if let Some(command_idx) = self.get_command_idx(row, column, gs) {
                    self.state.select(command_idx, self.commands.len());
                    self.force_render(gs);
                }
            }
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => {
                if let Some(command_idx) = self.get_command_idx(row, column, gs) {
                    match self.commands.remove(command_idx).1.execute() {
                        CommandResult::Simple(event) => gs.event.push(event),
                        CommandResult::BigCB(cb) => cb(gs, ws, tree, term),
                    };
                    return Status::Finished;
                }
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.commands.len());
                self.force_render(gs);
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.commands.len());
                self.force_render(gs);
            }
            _ => (),
        };
        Status::Pending
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

    fn resize_success(&mut self, _: &mut GlobalState) -> bool {
        true
    }

    fn render(&mut self, _: &mut GlobalState) {}
}

impl Pallet {
    pub fn new(git_tui: Option<String>) -> Self {
        let commands = [
            Some(Command::components("Open file", OpenFileSelector::run)),
            Some(Command::components("Open embeded terminal", change_state::open_embeded_terminal)),
            git_tui.map(|git_tui| Command::pass_event("Open Git TUI", IdiomEvent::EmbededApp(Some(git_tui)))),
            Some(Command::pass_event("Open terminal", IdiomEvent::EmbededApp(None))),
            Some(Command::components("LSP to bash", change_state::set_lsp)),
            Some(Command::components("UPPERCASE", formatting::uppercase)),
            Some(Command::components("LOWERCASE", formatting::lowercase)),
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

        Pallet { commands, pattern: TextField::new(String::new(), Some(true)), state: State::new() }
    }

    #[inline]
    pub fn run(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let git_tui = gs.git_tui.to_owned();
        if let Err(error) = Pallet::new(git_tui).run(gs, ws, tree, term) {
            gs.error(error);
        };
    }

    #[inline]
    fn get_command_idx(&self, row: u16, column: u16, gs: &GlobalState) -> Option<usize> {
        let Position { row, .. } = Self::get_rect(gs).relative_position(row, column)?;
        let line = row as usize;
        let command_idx = self.state.at_line + line.checked_sub(1)?;
        if self.commands.len() <= command_idx {
            return None;
        }
        Some(command_idx)
    }

    pub fn get_rect(gs: &GlobalState) -> Rect {
        gs.screen_rect.top(15).vcenter(100).with_borders()
    }
}

#[cfg(test)]
mod tests;
