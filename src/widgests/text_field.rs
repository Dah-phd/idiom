use crate::global_state::{Clipboard, PopupMessage, TreeEvent, WorkspaceEvent};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use super::count_as_string;

const CURSOR: Style = Style {
    fg: None,
    bg: None,
    underline_color: None,
    add_modifier: Modifier::REVERSED,
    sub_modifier: Modifier::empty(),
};

const SELECT: Style = Style {
    fg: None,
    bg: Some(Color::Rgb(72, 72, 72)),
    underline_color: None,
    add_modifier: Modifier::empty(),
    sub_modifier: Modifier::empty(),
};

#[derive(Default)]
pub struct TextField<T: Default + Clone> {
    pub text: String,
    char: usize,
    select: Option<(usize, usize)>,
    on_text_update: Option<T>,
}

impl<T: Default + Clone> TextField<T> {
    pub fn new(text: String, on_text_update: Option<T>) -> Self {
        Self { char: text.len(), text, select: None, on_text_update }
    }

    pub fn take_text(&mut self) -> String {
        self.char = 0;
        self.select = None;
        std::mem::take(&mut self.text)
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

    pub fn insert_formatted_text(&self, buffer: &mut Vec<Span<'static>>) {
        match self.select.as_ref().map(|(f, t)| if f > t { (*t, *f) } else { (*f, *t) }) {
            Some((from, to)) => self.text_cursor_select(from, to, buffer),
            None => self.text_cursor(buffer),
        }
    }

    fn text_cursor(&self, buffer: &mut Vec<Span<'static>>) {
        if self.char == self.text.len() {
            buffer.push(Span::raw(self.text.to_owned()));
            buffer.push(Span::styled(" ", CURSOR));
        } else {
            buffer.push(Span::raw(self.text[..self.char].to_owned()));
            buffer.push(Span::styled(self.text[self.char..=self.char].to_owned(), CURSOR));
            buffer.push(Span::raw(self.text[self.char + 1..].to_owned()));
        }
    }

    fn text_cursor_select(&self, from: usize, to: usize, buffer: &mut Vec<Span<'static>>) {
        buffer.push(Span::raw(self.text[..from].to_owned()));
        if from == self.char {
            buffer.push(Span::styled(self.text[self.char..=self.char].to_owned(), CURSOR));
            buffer.push(Span::styled(self.text[from + 1..to].to_owned(), SELECT));
            buffer.push(Span::raw(self.text[to..].to_owned()));
        } else if self.char == self.text.len() {
            buffer.push(Span::styled(self.text[from..to].to_owned(), SELECT));
            buffer.push(Span::raw(self.text[to..].to_owned()));
            buffer.push(Span::styled(" ", CURSOR));
        } else {
            buffer.push(Span::styled(self.text[from..to].to_owned(), SELECT));
            buffer.push(Span::styled(self.text[to..=to].to_owned(), CURSOR));
            buffer.push(Span::raw(self.text[to + 1..].to_owned()));
        }
    }

    pub fn map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> Option<T> {
        match key.code {
            KeyCode::Char('c' | 'C') if key.modifiers == KeyModifiers::CONTROL => {
                if let Some(clip) = self.get_selected() {
                    clipboard.push(clip);
                };
                Some(T::default())
            }
            KeyCode::Char('x' | 'X') if key.modifiers == KeyModifiers::CONTROL => {
                if let Some(clip) = self.take_selected() {
                    clipboard.push(clip);
                    return Some(self.on_text_update.clone().unwrap_or_default());
                };
                Some(T::default())
            }
            KeyCode::Char('v' | 'V') if key.modifiers == KeyModifiers::CONTROL => {
                if let Some(clip) = clipboard.pull() {
                    if !clip.contains('\n') {
                        self.take_selected();
                        self.text.insert_str(self.char, clip.as_str());
                        self.char += clip.len();
                        return Some(self.on_text_update.clone().unwrap_or_default());
                    };
                };
                Some(T::default())
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
                Some(T::default())
            }
            KeyCode::Left if key.modifiers == KeyModifiers::SHIFT => {
                self.init_select();
                self.char = self.char.saturating_sub(1);
                self.push_select();
                Some(T::default())
            }
            KeyCode::Left => {
                self.select = None;
                self.char = self.char.saturating_sub(1);
                Some(T::default())
            }
            KeyCode::Right if key.modifiers == KeyModifiers::SHIFT => {
                self.init_select();
                if self.text.len() > self.char {
                    self.char += 1;
                };
                self.push_select();
                Some(T::default())
            }
            KeyCode::Right => {
                self.select = None;
                if self.text.len() > self.char {
                    self.char += 1;
                };
                Some(T::default())
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

impl TextField<PopupMessage> {
    pub fn with_tree_access(text: String) -> Self {
        Self::new(text, Some(PopupMessage::Tree(TreeEvent::PopupAccess)))
    }

    pub fn with_editor_access(text: String) -> Self {
        Self::new(text, Some(PopupMessage::Workspace(WorkspaceEvent::PopupAccess)))
    }
}

impl TextField<()> {
    pub fn basic(text: String) -> Self {
        Self { char: text.len(), text, ..Default::default() }
    }
}
