mod snippets;
use super::ModalMessage;
use crate::{
    configs::EditorAction,
    global_state::GlobalState,
    render::{
        layout::{IterLines, Rect},
        state::State,
    },
    syntax::Lang,
    workspace::CursorPosition,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use lsp_types::CompletionItem;
use snippets::parse_completion_item;

pub struct AutoComplete {
    state: State,
    filter: String,
    filtered: Vec<(i64, usize)>,
    completions: Vec<CompletionItem>,
}

impl AutoComplete {
    pub fn new(completions: Vec<CompletionItem>, line: String, c: CursorPosition, matcher: &SkimMatcherV2) -> Self {
        let mut filter = String::new();
        for ch in line.chars().take(c.char) {
            if ch.is_alphabetic() || ch == '_' {
                filter.push(ch);
            } else {
                filter.clear();
            };
        }
        let mut modal = Self { state: State::new(), filter, filtered: Vec::new(), completions };
        modal.build_matches(matcher);
        modal
    }

    pub fn map(&mut self, action: EditorAction, lang: &Lang, gs: &mut GlobalState) -> ModalMessage {
        match action {
            EditorAction::NewLine | EditorAction::Indent => {
                let mut completion_item = self.completions.remove(self.filtered.remove(self.state.selected).1);
                if let Some(data) = completion_item.data.take() {
                    lang.handle_completion_data(data, gs);
                };
                gs.event.push(parse_completion_item(completion_item));
                ModalMessage::TakenDone
            }
            EditorAction::Char(ch) => self.push_filter(ch, &gs.matcher),
            EditorAction::Down => {
                self.state.next(self.filtered.len());
                ModalMessage::Taken
            }
            EditorAction::Up => {
                self.state.prev(self.filtered.len());
                ModalMessage::Taken
            }
            EditorAction::Backspace => self.filter_pop(&gs.matcher),
            _ => ModalMessage::Done,
        }
    }

    #[inline]
    pub fn render(&mut self, area: &Rect, gs: &mut GlobalState) {
        let backend = gs.backend();
        self.state.update_at_line(area.height as usize);
        let mut lines = area.into_iter();
        for (idx, item) in self.iter_filtered().enumerate().skip(self.state.at_line) {
            let Some(line) = lines.next() else { break };
            let mut builder = line.unsafe_builder(backend);
            match item.detail.as_ref() {
                Some(detail) => {
                    let mut components = [" ", &item.label, "  ", detail].into_iter();
                    _ = match idx == self.state.selected {
                        true => components.all(|text| builder.push_styled(text, self.state.highlight)),
                        false => components.all(|text| builder.push(text)),
                    };
                }
                None => {
                    let mut components = [" ", &item.label].into_iter();
                    _ = match idx == self.state.selected {
                        true => components.all(|text| builder.push_styled(text, self.state.highlight)),
                        false => components.all(|text| builder.push(text)),
                    }
                }
            }
        }
        lines.clear_to_end(backend);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    fn filter_pop(&mut self, matcher: &SkimMatcherV2) -> ModalMessage {
        self.filter.pop();
        self.build_matches(matcher);
        if self.filter.is_empty() {
            return ModalMessage::Done;
        }
        self.filtered.as_slice().into()
    }

    fn push_filter(&mut self, ch: char, matcher: &SkimMatcherV2) -> ModalMessage {
        if ch.is_alphabetic() || ch == '_' {
            self.filter.push(ch);
            self.build_matches(matcher);
            self.filtered.as_slice().into()
        } else {
            ModalMessage::Done
        }
    }

    fn build_matches(&mut self, matcher: &SkimMatcherV2) {
        self.filtered = self
            .completions
            .iter()
            .enumerate()
            .filter_map(|(item_idx, item)| {
                matcher.fuzzy_match(item.filter_text.as_ref().unwrap_or(&item.label), &self.filter).map(|score| {
                    let divisor = item.label.len().abs_diff(self.filter.len()) as i64;
                    let new_score = if divisor != 0 { score / divisor } else { score };
                    (new_score, item_idx)
                })
            })
            .collect();
        self.filtered.sort_by(|(idx, _), (rhidx, _)| rhidx.cmp(idx));
        self.state.reset();
    }

    fn iter_filtered(&self) -> impl Iterator<Item = &CompletionItem> {
        self.filtered.iter().map(|(.., idx)| &self.completions[*idx])
    }
}

#[cfg(test)]
mod tests;
