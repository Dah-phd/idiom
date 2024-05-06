use crate::{
    global_state::{Clipboard, PopupMessage, TreeEvent, WorkspaceEvent},
    render::backend::{color, Backend, Style},
};
use core::ops::Range;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    count_as_string,
    layout::{Line, LineBuilder},
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

    pub fn text_set(&mut self, text: String) {
        self.select = None;
        self.text = text;
        self.char = self.text.len();
    }

    pub fn text_take(&mut self) -> String {
        self.char = 0;
        self.select = None;
        std::mem::take(&mut self.text)
    }

    pub fn text_get_token_at_cursor(&self) -> Option<&str> {
        let token_range = arg_range_at(&self.text, self.char);
        self.text.get(token_range)
    }

    pub fn text_replace_token(&mut self, new: &str) {
        let token_range = arg_range_at(&self.text, self.char);
        self.char = new.len() + token_range.start;
        self.select = None;
        self.text.replace_range(token_range, new);
    }

    /// returns blockless paragraph widget " >> inner text"
    pub fn widget(&self, line: Line, backend: &mut Backend) -> std::io::Result<()> {
        let mut builder = line.unsafe_builder(backend)?;
        builder.push(" >> ")?;
        self.insert_formatted_text(builder)
    }

    /// returns blockless paragraph widget "99+ >> inner text"
    pub fn widget_with_count(&self, line: Line, count: usize, backend: &mut Backend) -> std::io::Result<()> {
        let mut builder = line.unsafe_builder(backend)?;
        builder.push(count_as_string(count).as_str())?;
        builder.push(" >> ")?;
        self.insert_formatted_text(builder)
    }

    pub fn insert_formatted_text(&self, line_builder: LineBuilder) -> std::io::Result<()> {
        match self.select.as_ref().map(|(f, t)| if f > t { (*t, *f) } else { (*f, *t) }) {
            Some((from, to)) => self.text_cursor_select(from, to, line_builder),
            None => self.text_cursor(line_builder),
        }
    }

    fn text_cursor(&self, mut builder: LineBuilder) -> std::io::Result<()> {
        if self.char == self.text.len() {
            builder.push(&self.text)?;
            builder.push_styled(" ", Style::reversed())?;
        } else {
            builder.push(self.text[..self.char].as_ref())?;
            builder.push_styled(self.text[self.char..=self.char].as_ref(), Style::reversed())?;
            builder.push(self.text[self.char + 1..].as_ref())?;
        };
        Ok(())
    }

    fn text_cursor_select(&self, from: usize, to: usize, mut builder: LineBuilder) -> std::io::Result<()> {
        builder.push(self.text[..from].as_ref())?;
        if from == self.char {
            builder.push_styled(self.text[self.char..=self.char].as_ref(), Style::reversed())?;
            builder.push_styled(self.text[from + 1..to].as_ref(), Style::bg(color::rgb(72, 72, 72)))?;
            builder.push(self.text[to..].as_ref())?;
        } else if self.char == self.text.len() {
            builder.push_styled(self.text[from..to].as_ref(), Style::bg(color::rgb(72, 72, 72)))?;
            builder.push(self.text[to..].as_ref())?;
            builder.push_styled(" ", Style::reversed())?;
        } else {
            builder.push_styled(self.text[from..to].as_ref(), Style::bg(color::rgb(72, 72, 72)))?;
            builder.push_styled(self.text[to..=to].as_ref(), Style::reversed())?;
            builder.push(self.text[to + 1..].as_ref())?;
        }
        Ok(())
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
            KeyCode::Left => self.move_left(key.modifiers),
            KeyCode::Right => self.move_right(key.modifiers),
            _ => None,
        }
    }

    fn move_left(&mut self, mods: KeyModifiers) -> Option<T> {
        let should_select = mods.contains(KeyModifiers::SHIFT);
        if should_select {
            self.init_select();
        } else {
            self.select = None;
        };
        self.char = self.char.saturating_sub(1);
        if mods.contains(KeyModifiers::CONTROL) {
            // jump
            while self.char > 0 {
                let next_idx = self.char - 1;
                if matches!(self.text.chars().nth(next_idx), Some(ch) if !ch.is_alphabetic() && !ch.is_numeric()) {
                    break;
                };
                self.char = next_idx;
            }
        };
        if should_select {
            self.push_select();
        };
        Some(T::default())
    }

    fn move_right(&mut self, mods: KeyModifiers) -> Option<T> {
        let should_select = mods.contains(KeyModifiers::SHIFT);
        if should_select {
            self.init_select();
        } else {
            self.select = None;
        };
        self.char = std::cmp::min(self.text.len(), self.char + 1);
        if mods.contains(KeyModifiers::CONTROL) {
            // jump
            while self.text.len() > self.char {
                self.char += 1;
                if matches!(self.text.chars().nth(self.char), Some(ch) if !ch.is_alphabetic() && !ch.is_numeric()) {
                    break;
                }
            }
        };
        if should_select {
            self.push_select();
        };
        Some(T::default())
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

pub fn arg_range_at(line: &str, idx: usize) -> Range<usize> {
    let mut token_start = 0;
    let mut last_not_in_token = false;
    for (char_idx, ch) in line.char_indices() {
        if !ch.is_whitespace() {
            if last_not_in_token {
                token_start = char_idx;
            }
            last_not_in_token = false;
        } else if char_idx >= idx {
            if last_not_in_token {
                return idx..idx;
            }
            return token_start..char_idx;
        } else {
            last_not_in_token = true;
        }
    }
    if idx < line.len() {
        token_start..line.len()
    } else if !last_not_in_token && token_start <= idx {
        token_start..idx
    } else {
        idx..idx
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

#[cfg(test)]
mod test {
    use super::TextField;
    use crate::global_state::Clipboard;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_setting() {
        let mut field: TextField<()> = TextField::default();
        field.text_set("12345".to_owned());
        assert_eq!(&field.text, "12345");
        assert_eq!(field.char, 5);
        let mut clip = Clipboard::default();
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT), &mut clip);
        assert!(field.select.is_some());
        assert_eq!(field.char, 4);
        assert_eq!(&field.text_take(), "12345");
        assert_eq!(field.char, 0);
        assert_eq!(&field.text, "");
        assert!(field.select.is_none());
    }

    #[test]
    fn test_move() {
        let mut field: TextField<()> = TextField::default();
        let mut clip = Clipboard::default();
        field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty()), &mut clip);
        assert!(field.char == 0);
        field.text_set("12".to_owned());
        field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL), &mut clip);
        assert_eq!(field.char, 2);
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL), &mut clip);
        assert_eq!(field.char, 0);
        field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty()), &mut clip);
        assert_eq!(field.char, 1);
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::empty()), &mut clip);
        assert_eq!(field.char, 0);
    }

    #[test]
    fn test_select() {
        let mut field: TextField<()> = TextField::default();
        let mut clip = Clipboard::default();
        field.text_set("a3cde".to_owned());
        field.char = 0;
        field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::SHIFT), &mut clip);
        assert_eq!(field.select, Some((0, 5)));
        assert_eq!(field.char, 5);
        field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty()), &mut clip);
        assert!(field.select.is_none());
        assert_eq!(field.char, 5);
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT), &mut clip);
        assert_eq!(field.select, Some((5, 4)));
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::CONTROL), &mut clip);
        assert_eq!(field.select, Some((5, 0)));
        field.map(&KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()), &mut clip);
        assert_eq!(&field.text, "");
    }
}
