use super::ModalMessage;
#[cfg(build = "debug")]
use crate::debug_to_file;
use crate::{
    global_state::GlobalState,
    render::WrappedState,
    syntax::line_builder::Lang,
    utils::{BORDERED_BLOCK, REVERSED},
};
use crossterm::event::{KeyCode, KeyEvent};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use lsp_types::CompletionItem;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

pub struct AutoComplete {
    state: WrappedState,
    filter: String,
    matcher: SkimMatcherV2,
    filtered: Vec<(Line<'static>, i64, usize)>,
    completions: Vec<CompletionItem>,
}

impl AutoComplete {
    pub fn new(completions: Vec<CompletionItem>, line: String, idx: usize) -> Self {
        let mut filter = String::new();
        for ch in line[..idx].chars() {
            if ch.is_alphabetic() || ch == '_' {
                filter.push(ch);
            } else {
                filter.clear();
            }
        }
        let mut modal = Self {
            state: WrappedState::default(),
            filter,
            matcher: SkimMatcherV2::default(),
            filtered: Vec::new(),
            completions,
        };
        modal.build_matches();
        modal
    }

    pub fn map(&mut self, key: &KeyEvent, lang: &Lang, gs: &mut GlobalState) -> ModalMessage {
        match key.code {
            KeyCode::Enter | KeyCode::Tab => {
                if let Some(idx) = self.state.selected() {
                    let mut filtered_completion = self.completions.remove(self.filtered.remove(idx).2);
                    #[cfg(build = "debug")]
                    debug_to_file("test_data.comp", filtered_completion);
                    if let Some(data) = filtered_completion.data.take() {
                        lang.handle_completion_data(data, gs);
                    };
                    gs.workspace.push(filtered_completion.into());
                }
                ModalMessage::TakenDone
            }
            KeyCode::Char(ch) => self.push_filter(ch),
            KeyCode::Down => {
                self.state.next(&self.filtered);
                ModalMessage::Taken
            }
            KeyCode::Up => {
                self.state.prev(&self.filtered);
                ModalMessage::Taken
            }
            KeyCode::Backspace => self.filter_pop(),
            _ => ModalMessage::Done,
        }
    }

    pub fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        let complitions = self.filtered.iter().map(|(line, ..)| ListItem::new(line.clone())).collect::<Vec<_>>();
        frame.render_stateful_widget(
            List::new(complitions).block(BORDERED_BLOCK).highlight_style(REVERSED),
            area,
            self.state.get(),
        );
    }

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
                    let mut line = vec![Span::from(item.label.to_owned())];
                    if let Some(info) = item.detail.as_ref() {
                        line.push(Span::styled(format!("  {info}"), Style::default().add_modifier(Modifier::DIM)));
                    };
                    (Line::from(line), new_score, item_idx)
                })
            })
            .collect();
        self.filtered.sort_by(|(_, idx, _), (_, rhidx, _)| rhidx.cmp(idx));
        self.state.set(0);
    }
}
