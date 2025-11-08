mod change_state;
mod formatting;
use super::{popup_file_open::OpenFileSelector, Command, CommandResult, Components, Popup, Status};
use crate::{
    configs::{EDITOR_CFG_FILE, KEY_MAP, THEME_FILE, THEME_UI},
    embeded_term::EditorTerminal,
    ext_tui::{text_field::map_key, State, StyleExt},
    global_state::{GlobalState, IdiomEvent},
    tree::Tree,
    workspace::{line::EditorLine, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::ContentStyle;
use fuzzy_matcher::FuzzyMatcher;
use idiom_tui::{
    layout::Rect,
    text_field::{Status as InputStatus, TextField},
    Position,
};
use std::path::PathBuf;
use std::process::{Command as SysCommand, Stdio};

enum Mode {
    Cmd,
    EasyAccess,
}

pub struct Pallet {
    commands: Vec<(i64, Command)>,
    pattern: TextField,
    state: State,
    mode: Mode,
}

impl Popup for Pallet {
    fn force_render(&mut self, gs: &mut GlobalState) {
        match self.mode {
            Mode::EasyAccess => self.force_render_as_pallet(gs),
            Mode::Cmd => self.force_render_as_cmd(gs),
        }
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut super::Components) -> Status {
        let Components { gs, ws, tree, term } = components;

        if self.commands.is_empty() {
            return Status::Finished;
        }

        match key.code {
            KeyCode::Enter => {
                match self.mode {
                    Mode::EasyAccess => match self.commands.remove(self.state.selected).1.execute() {
                        CommandResult::Simple(event) => gs.event.push(event),
                        CommandResult::BigCB(cb) => cb(gs, ws, tree, term),
                    },
                    Mode::Cmd => {
                        if self.pattern.as_str() == "ss" {
                            if let Some(editor) = ws.get_active() {
                                editor.select_scope();
                            }
                        } else if self.pattern.as_str().starts_with("e|") {
                            let full_cmd = &self.pattern.as_str()[2..];
                            if full_cmd.is_empty() {
                                return Status::Finished;
                            }
                            let name: String = full_cmd
                                .chars()
                                .map(|c| if c.is_ascii_alphabetic() || c.is_ascii_digit() { c } else { '_' })
                                .collect();

                            let mut cmd_split = full_cmd.split(" ");
                            let Some(cmd) = cmd_split.next() else {
                                return Status::Finished;
                            };
                            match PathBuf::from("./").canonicalize() {
                                Ok(base_path) => {
                                    let mut path = base_path.clone();
                                    path.push(format!("{name}.out"));
                                    let mut id = 0_usize;
                                    while path.exists() {
                                        path = base_path.clone();
                                        path.push(format!("{name}_{id}.out"));
                                        id += 1;
                                    }
                                    let child = SysCommand::new(cmd)
                                        .args(cmd_split)
                                        .stdout(Stdio::piped())
                                        .stderr(Stdio::piped())
                                        .spawn();

                                    match child.and_then(|c| c.wait_with_output()) {
                                        Ok(out) => {
                                            let mut content = vec![];
                                            if out.status.success() {
                                                // adds errors on top
                                                content.extend(
                                                    String::from_utf8_lossy(&out.stderr).lines().map(EditorLine::from),
                                                );
                                                content.extend(
                                                    String::from_utf8_lossy(&out.stdout).lines().map(EditorLine::from),
                                                );
                                            } else {
                                                // adds out on top
                                                content.extend(
                                                    String::from_utf8_lossy(&out.stdout).lines().map(EditorLine::from),
                                                );
                                                content.extend(
                                                    String::from_utf8_lossy(&out.stderr).lines().map(EditorLine::from),
                                                );
                                            }
                                            if !content.is_empty() {
                                                ws.new_text_from_data(path, content, None, gs);
                                            }
                                        }
                                        Err(error) => gs.error(error),
                                    }
                                }
                                Err(error) => gs.error(error),
                            }
                        }
                    }
                }
                return Status::Finished;
            }
            KeyCode::Up => {
                self.state.prev(self.commands.len());
            }
            KeyCode::Down => {
                self.state.next(self.commands.len());
            }
            KeyCode::Backspace if self.pattern.is_empty() && matches!(self.mode, Mode::Cmd) => {
                self.mode = Mode::EasyAccess;
                gs.draw(ws, tree, term);
                gs.force_screen_rebuild();
            }
            KeyCode::Char(':') if self.pattern.is_empty() && matches!(self.mode, Mode::EasyAccess) => {
                self.mode = Mode::Cmd;
                gs.draw(ws, tree, term);
                gs.force_screen_rebuild();
            }
            _ => {
                match map_key(&mut self.pattern, key, &mut gs.clipboard) {
                    Some(InputStatus::Skipped) | None => {}
                    Some(InputStatus::UpdatedCursor) => self.force_render(gs),
                    Some(InputStatus::Updated) => {
                        self.sort_commands_by_pattern(gs);
                        self.force_render(gs);
                    }
                }
                return Status::Pending;
            }
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
        if !self.pattern.paste_passthrough(clip).is_updated() {
            return false;
        }
        self.sort_commands_by_pattern(components.gs);
        true
    }

    fn resize_success(&mut self, _: &mut GlobalState) -> bool {
        true
    }

    fn render(&mut self, _: &mut GlobalState) {}
}

impl Pallet {
    fn new(git_tui: Option<String>, mode: Mode) -> Self {
        let commands = [
            Some(Command::components("Open file", OpenFileSelector::run)),
            Some(Command::components("Open embeded terminal", change_state::open_embeded_terminal)),
            git_tui.map(|git_tui| Command::pass_event("Open Git TUI", IdiomEvent::EmbededApp(Some(git_tui)))),
            Some(Command::pass_event("Open terminal", IdiomEvent::EmbededApp(None))),
            Some(Command::components("Select LSP", change_state::select_lsp)),
            Some(Command::components("UPPERCASE", formatting::uppercase)),
            Some(Command::components("lowercase", formatting::lowercase)),
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

        Pallet { commands, pattern: TextField::default(), state: State::new(), mode }
    }

    pub fn run(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let git_tui = gs.git_tui.to_owned();
        if let Err(error) = Pallet::new(git_tui, Mode::EasyAccess).main_loop(gs, ws, tree, term) {
            gs.error(error);
        }
    }

    pub fn run_as_command(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let git_tui = gs.git_tui.to_owned();
        let mut pallet = Pallet::new(git_tui, Mode::Cmd);
        if let Err(error) = pallet.main_loop(gs, ws, tree, term) {
            gs.error(error);
        }
    }

    fn sort_commands_by_pattern(&mut self, gs: &GlobalState) {
        for (score, cmd) in self.commands.iter_mut() {
            *score = match gs.matcher.fuzzy_match(cmd.label, self.pattern.as_str()) {
                Some(new_score) => new_score,
                None => i64::MAX,
            };
        }
        self.state.select(0, self.commands.len());
        self.commands.sort_by(|(score, _), (rhscore, _)| score.cmp(rhscore));
    }

    fn get_command_idx(&self, row: u16, column: u16, gs: &GlobalState) -> Option<usize> {
        if matches!(self.mode, Mode::Cmd) {
            return None;
        }
        let Position { row, .. } = Self::get_pallet_rect(gs).relative_position(row, column)?;
        let line = row as usize;
        let command_idx = self.state.at_line + line.checked_sub(1)?;
        if self.commands.len() <= command_idx {
            return None;
        }
        Some(command_idx)
    }

    fn force_render_as_pallet(&mut self, gs: &mut GlobalState) {
        let mut rect = Self::get_pallet_rect(gs);
        rect.draw_borders(None, None, gs.backend());

        let Some(line) = rect.next_line() else { return };
        self.pattern.widget(line, ContentStyle::reversed(), gs.get_select_style(), gs.backend());

        let options = self.commands.iter().map(|cmd| cmd.1.label);
        self.state.render_list(options, rect, gs.backend());
    }

    fn force_render_as_cmd(&mut self, gs: &mut GlobalState) {
        let rect = Self::get_cmd_rect(gs);
        rect.draw_borders(None, None, gs.backend());
        let mut lines = rect.into_iter();

        let Some(line) = lines.next() else { return };

        let select = gs.get_select_style();
        let mut line_builder = line.unsafe_builder(gs.backend());
        line_builder.push(" : ");
        self.pattern.insert_formatted_text(line_builder, ContentStyle::reversed(), select);

        let Some(line) = lines.next() else { return };
        line.render("resolution", gs.backend());
    }

    pub fn get_pallet_rect(gs: &GlobalState) -> Rect {
        gs.screen().top(15).vcenter(100).with_borders()
    }

    pub fn get_cmd_rect(gs: &GlobalState) -> Rect {
        gs.screen().top(4).vcenter(100).with_borders()
    }
}

#[cfg(test)]
mod tests;
