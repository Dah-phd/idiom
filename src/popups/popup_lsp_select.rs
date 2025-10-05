use super::{Components, Popup, Status};
use crate::{
    configs::FileType,
    embeded_term::EditorTerminal,
    ext_tui::{text_field::map_key, State, StyleExt},
    global_state::{GlobalState, IdiomEvent},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::ContentStyle;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use idiom_tui::{
    layout::{IterLines, Rect},
    text_field::{Status as InputStatus, TextField},
    Position,
};

pub struct SelectorLSP {
    pattern: TextField,
    state: State,
    file_types: Vec<(i64, &'static str, FileType)>,
}

impl SelectorLSP {
    pub fn run(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let file_types = FileType::iter_langs()
            .into_iter()
            .map(|x| (0, x.into(), x))
            .chain([(0, "markdown", FileType::MarkDown), (0, "no LSP", FileType::Text)])
            .collect();
        let pattern = TextField::default();
        let mut new = Self { pattern, state: State::default(), file_types };

        if let Err(error) = new.run(gs, ws, tree, term) {
            gs.error(error);
        };
    }

    fn finish(&mut self) -> FileType {
        self.file_types[self.state.selected].2
    }

    fn filter(&mut self, matcher: &SkimMatcherV2) {
        self.state.select(0, self.file_types.len());
        for (score, label, ..) in self.file_types.iter_mut() {
            *score = matcher.fuzzy_match(label, self.pattern.as_str()).unwrap_or_default();
        }
        self.file_types.sort_by(|(idx, ..), (rhidx, ..)| rhidx.cmp(idx));
    }

    fn get_idx_from_rect(&self, row: u16, column: u16, gs: &GlobalState) -> Option<usize> {
        let Position { row, .. } = Self::get_rect(gs).relative_position(row, column)?;
        let line = row as usize;
        let path_index = self.state.at_line + line.checked_sub(1)?;
        if self.file_types.len() <= path_index {
            return None;
        }
        Some(path_index)
    }

    fn get_rect(gs: &GlobalState) -> Rect {
        gs.screen().top(15).vcenter(100).with_borders()
    }
}

impl Popup for SelectorLSP {
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
        for (idx, (score, text, ..)) in self.file_types.iter().enumerate().skip(self.state.at_line) {
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
                self.state.prev(self.file_types.len());
            }
            KeyEvent { code: KeyCode::Down, .. } => {
                self.state.next(self.file_types.len());
            }
            KeyEvent { code: KeyCode::Enter, .. } => {
                gs.event.push(IdiomEvent::SetLSP(self.finish()));
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
                    self.state.select(path_idx, self.file_types.len());
                    self.force_render(gs);
                }
                Status::Pending
            }
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => {
                if let Some(idx) = self.get_idx_from_rect(row, column, gs) {
                    self.state.select(idx, self.file_types.len());
                    gs.event.push(IdiomEvent::SetLSP(self.finish()));
                    return Status::Finished;
                }
                Status::Pending
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.file_types.len());
                self.force_render(gs);
                Status::Pending
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.file_types.len());
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
