use crate::global_state::{Clipboard, PopupMessage, TreeEvent, WorkspaceEvent};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use super::count_as_string;

#[derive(Default)]
pub struct TextField {
    pub text: String,
    char: usize,
    select: Option<(usize, usize)>,
    on_text_update: Option<PopupMessage>,
}

impl TextField {
    pub fn with_tree_access() -> Self {
        Self { on_text_update: Some(PopupMessage::Tree(TreeEvent::PopupAccess)), ..Default::default() }
    }

    pub fn with_editor_access() -> Self {
        Self { on_text_update: Some(PopupMessage::Workspace(WorkspaceEvent::PopupAccess)), ..Default::default() }
    }

    /// returns blockless paragraph widget " >> inner text"
    pub fn widget(&self) -> Paragraph<'static> {
        let mut buffer = vec![Span::raw(" >> ")];
        self.insert_formatted_text(&mut buffer);
        Paragraph::new(Line::from(buffer))
    }

    /// returns blockless paragraph widget "99+ >> inner text"
    pub fn widget_with_count(&self, count: usize) -> Paragraph<'static> {
        let mut buffer = vec![Span::raw(count_as_string(count)), Span::raw(" >> ")];
        self.insert_formatted_text(&mut buffer);
        Paragraph::new(Line::from(buffer))
    }

    fn insert_formatted_text(&self, buffer: &mut Vec<Span<'static>>) {
        match self.select.as_ref().map(|(f, t)| if f > t { (t, f) } else { (f, t) }) {
            Some((from, to)) => {
                buffer.push(Span::raw(self.text[..*from].to_owned()));
                buffer.push(Span::styled(
                    self.text[*from..*to].to_owned(),
                    Style { bg: Some(Color::Rgb(72, 72, 72)), ..Default::default() },
                ));
                buffer.push(Span::raw(self.text[*to..].to_owned()));
            }
            None => buffer.push(Span::raw(self.text.to_owned())),
        }
        buffer.push(Span::styled(" ", Style { bg: Some(Color::White), ..Default::default() }));
    }

    pub fn map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> Option<PopupMessage> {
        match key.code {
            KeyCode::Char('c' | 'C') if key.modifiers == KeyModifiers::CONTROL => {
                if let Some(clip) = self.get_selected() {
                    clipboard.push(clip);
                };
                Some(PopupMessage::default())
            }
            KeyCode::Char('x' | 'X') if key.modifiers == KeyModifiers::CONTROL => {
                if let Some(clip) = self.take_selected() {
                    clipboard.push(clip);
                    return Some(self.on_text_update.clone().unwrap_or_default());
                };
                Some(PopupMessage::default())
            }
            KeyCode::Char('v' | 'V') if key.modifiers == KeyModifiers::CONTROL => {
                if let Some(clip) = clipboard.pull() {
                    self.take_selected();
                    self.text.insert_str(self.char, clip.as_str());
                    return Some(self.on_text_update.clone().unwrap_or_default());
                };
                Some(PopupMessage::default())
            }
            KeyCode::Char(ch) => {
                self.take_selected();
                self.text.insert(self.char, ch);
                self.char += 1;
                Some(self.on_text_update.clone().unwrap_or_default())
            }
            KeyCode::Delete => {
                if self.take_selected().is_some() {
                } else if self.char < self.text.len() && !self.text.is_empty() {
                    self.text.remove(self.char);
                };
                Some(self.on_text_update.clone().unwrap_or_default())
            }
            KeyCode::Backspace => {
                if self.take_selected().is_some() {
                } else if self.char > 0 && !self.text.is_empty() {
                    self.char -= 1;
                    self.text.remove(self.char);
                };
                Some(self.on_text_update.clone().unwrap_or_default())
            }
            KeyCode::End => {
                self.char = self.text.len().saturating_sub(1);
                Some(PopupMessage::default())
            }
            KeyCode::Left if key.modifiers == KeyModifiers::SHIFT => {
                self.init_select();
                self.char = self.char.saturating_sub(1);
                self.push_select();
                Some(PopupMessage::default())
            }
            KeyCode::Left => {
                self.select = None;
                self.char = self.char.saturating_sub(1);
                Some(PopupMessage::default())
            }
            KeyCode::Right if key.modifiers == KeyModifiers::SHIFT => {
                self.init_select();
                if self.text.len() > self.char {
                    self.char += 1;
                };
                self.push_select();
                Some(PopupMessage::default())
            }
            KeyCode::Right => {
                self.select = None;
                if self.text.len() > self.char {
                    self.char += 1;
                };
                Some(PopupMessage::default())
            }
            _ => None,
        }
    }

    fn init_select(&mut self) {
        if self.select.is_none() {
            self.select = Some((self.char, self.char))
        }
    }

    fn push_select(&mut self) {
        if let Some((_, to)) = self.select.as_mut() {
            *to = self.char;
        }
    }

    fn get_selected(&mut self) -> Option<String> {
        let (from, to) = self.select.map(|(f, t)| if f > t { (t, f) } else { (f, t) })?;
        if from == to {
            return None;
        }
        Some(self.text[from..to].to_owned())
    }

    fn take_selected(&mut self) -> Option<String> {
        let (from, to) = self.select.take().map(|(f, t)| if f > t { (t, f) } else { (f, t) })?;
        if from == to {
            return None;
        }
        let clip = self.text[from..to].to_owned();
        self.text.replace_range(from..to, "");
        self.char = from;
        Some(clip)
    }
}
