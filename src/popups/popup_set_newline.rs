use super::{Components, Popup, Status};
use crate::{
    editor_line::{CARRIAGE_NLINE, LineEnd, MSDOS_NLINE, POSIX_NLINE, RISCOS_NLINE},
    embeded_term::EditorTerminal,
    ext_tui::{State, StyleExt, text_field::map_key},
    global_state::GlobalState,
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::ContentStyle;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use idiom_tui::{
    Position,
    layout::{IterLines, Rect},
    text_field::{Status as InputStatus, TextField},
};

pub struct SetCustomNewLine {
    pattern: TextField,
    state: State,
    line_ends: Vec<(i64, &'static str, LineEnd)>,
}

impl SetCustomNewLine {
    pub fn run(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        if ws.get_active().is_none() {
            gs.message("No editors opened ...");
            return;
        }
        let line_ends = vec![
            (0, r#"POSIX \n"#, POSIX_NLINE),
            (0, r#"CARRIAGE \r"#, CARRIAGE_NLINE),
            (0, r#"MS \r\n"#, MSDOS_NLINE),
            (0, r#"RISC \n\r"#, RISCOS_NLINE),
        ];
        let pattern = TextField::default();
        let mut new = Self { pattern, state: State::default(), line_ends };

        if let Err(error) = new.main_loop(gs, ws, tree, term) {
            gs.error(error);
        };
    }

    fn finish(&mut self, ws: &mut Workspace) {
        let Some(editor) = ws.get_active() else { return };
        editor.set_line_end(self.line_ends.remove(self.state.selected).2);
    }

    fn filter(&mut self, matcher: &SkimMatcherV2) {
        self.state.select(0, self.line_ends.len());
        for (score, label, ..) in self.line_ends.iter_mut() {
            *score = matcher.fuzzy_match(label, self.pattern.as_str()).unwrap_or_default();
        }
        self.line_ends.sort_by(|(idx, ..), (rhidx, ..)| rhidx.cmp(idx));
    }

    fn get_idx_from_rect(&self, row: u16, column: u16, gs: &GlobalState) -> Option<usize> {
        let Position { row, .. } = Self::get_rect(gs).relative_position(row, column)?;
        let line = row as usize;
        let path_index = self.state.at_line + line.checked_sub(1)?;
        if self.line_ends.len() <= path_index {
            return None;
        }
        Some(path_index)
    }

    fn get_rect(gs: &GlobalState) -> Rect {
        gs.screen().top(15).vcenter(100).with_borders()
    }
}

impl Popup for SetCustomNewLine {
    fn force_render(&mut self, gs: &mut GlobalState) {
        let mut rect = Self::get_rect(gs);
        let accent = ContentStyle::fg(gs.ui_theme.accent());
        rect.draw_borders(None, None, gs.backend());
        match rect.next_line() {
            Some(line) => self.pattern.widget(line, ContentStyle::reversed(), gs.get_select_style(), gs.backend()),
            None => return,
        }
        self.state.update_at_line(rect.height as usize);
        let mut lines = rect.into_iter();
        for (idx, (score, text, ..)) in self.line_ends.iter().enumerate().skip(self.state.at_line) {
            let Some(line) = lines.next() else { break };
            match idx == self.state.selected {
                true => line.render_styled(text, self.state.highlight, gs.backend()),
                false => {
                    if *score == 0 {
                        line.render_styled(text, accent, gs.backend());
                    } else {
                        line.render(text, gs.backend())
                    }
                }
            }
        }
        lines.clear_to_end(gs.backend());
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status {
        let Components { gs, .. } = components;

        match key {
            KeyEvent { code: KeyCode::Up, .. } => {
                self.state.prev(self.line_ends.len());
            }
            KeyEvent { code: KeyCode::Down, .. } => {
                self.state.next(self.line_ends.len());
            }
            KeyEvent { code: KeyCode::Enter, .. } => {
                self.finish(components.ws);
                return Status::Finished;
            }
            _ => {
                match map_key(&mut self.pattern, key, &mut gs.clipboard) {
                    Some(InputStatus::Skipped) | None => {}
                    Some(InputStatus::Updated) => {
                        self.filter(&gs.matcher);
                        self.force_render(gs)
                    }
                    Some(InputStatus::UpdatedCursor) => self.force_render(gs),
                }
                return Status::Pending;
            }
        }
        self.force_render(gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status {
        let Components { gs, .. } = components;

        match event {
            MouseEvent { kind: MouseEventKind::Moved, column, row, .. } => {
                if let Some(path_idx) = self.get_idx_from_rect(row, column, gs) {
                    self.state.select(path_idx, self.line_ends.len());
                    self.force_render(gs);
                }
                Status::Pending
            }
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => {
                if let Some(idx) = self.get_idx_from_rect(row, column, gs) {
                    self.state.select(idx, self.line_ends.len());
                    self.finish(components.ws);
                    return Status::Finished;
                }
                Status::Pending
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.line_ends.len());
                self.force_render(gs);
                Status::Pending
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.line_ends.len());
                self.force_render(gs);
                Status::Pending
            }
            _ => Status::Pending,
        }
    }

    fn render(&mut self, _: &mut GlobalState) {}

    fn paste_passthrough(&mut self, clip: String, c: &mut Components) -> bool {
        if !self.pattern.paste_passthrough(clip).is_updated() {
            return false;
        }
        self.filter(&c.gs.matcher);
        true
    }

    fn resize_success(&mut self, _: &mut GlobalState) -> bool {
        true
    }
}
