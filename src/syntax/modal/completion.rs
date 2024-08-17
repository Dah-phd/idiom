use super::ModalMessage;
use crate::{
    global_state::GlobalState,
    render::{layout::Rect, state::State},
    syntax::Lang,
    workspace::CursorPosition,
};
use crossterm::event::{KeyCode, KeyEvent};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use lsp_types::CompletionItem;

pub struct AutoComplete {
    state: State,
    filter: String,
    matcher: SkimMatcherV2,
    filtered: Vec<(String, i64, usize)>,
    completions: Vec<CompletionItem>,
}

impl AutoComplete {
    pub fn new(completions: Vec<CompletionItem>, line: String, c: CursorPosition) -> Self {
        let mut filter = String::new();
        for ch in line.chars().take(c.char) {
            if ch.is_alphabetic() || ch == '_' {
                filter.push(ch);
            } else {
                filter.clear();
            };
        }
        let mut modal =
            Self { state: State::new(), filter, matcher: SkimMatcherV2::default(), filtered: Vec::new(), completions };
        modal.build_matches();
        modal
    }

    pub fn map(&mut self, key: &KeyEvent, lang: &Lang, gs: &mut GlobalState) -> ModalMessage {
        match key.code {
            KeyCode::Enter | KeyCode::Tab => {
                let mut filtered_completion = self.completions.remove(self.filtered.remove(self.state.selected).2);
                if let Some(data) = filtered_completion.data.take() {
                    lang.handle_completion_data(data, gs);
                };
                gs.workspace.push(filtered_completion.into());
                ModalMessage::TakenDone
            }
            KeyCode::Char(ch) => self.push_filter(ch),
            KeyCode::Down => {
                self.state.next(self.filtered.len());
                ModalMessage::Taken
            }
            KeyCode::Up => {
                self.state.prev(self.filtered.len());
                ModalMessage::Taken
            }
            KeyCode::Backspace => self.filter_pop(),
            _ => ModalMessage::Done,
        }
    }

    #[inline]
    pub fn render(&mut self, area: &Rect, gs: &mut GlobalState) {
        self.state.render_list(self.filtered.iter().map(|(c, ..)| c.as_str()), area, &mut gs.writer);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    fn filter_pop(&mut self) -> ModalMessage {
        self.filter.pop();
        self.build_matches();
        if self.filter.is_empty() {
            return ModalMessage::Done;
        }
        self.filtered.as_slice().into()
    }

    fn push_filter(&mut self, ch: char) -> ModalMessage {
        if ch.is_alphabetic() || ch == '_' {
            self.filter.push(ch);
            self.build_matches();
            self.filtered.as_slice().into()
        } else {
            ModalMessage::Done
        }
    }

    fn build_matches(&mut self) {
        self.filtered = self
            .completions
            .iter()
            .enumerate()
            .filter_map(|(item_idx, item)| {
                self.matcher.fuzzy_match(item.filter_text.as_ref().unwrap_or(&item.label), &self.filter).map(|score| {
                    let divisor = item.label.len().abs_diff(self.filter.len()) as i64;
                    let new_score = if divisor != 0 { score / divisor } else { score };
                    let mut line = format!(" {}", item.label);
                    if let Some(info) = item.detail.as_ref() {
                        line = format!("{line}  {info}");
                    };
                    (line, new_score, item_idx)
                })
            })
            .collect();
        self.filtered.sort_by(|(_, idx, _), (_, rhidx, _)| rhidx.cmp(idx));
        self.state.select(0, self.filtered.len());
    }
}
