use super::backend::{Backend, StyleExt};
use core::ops::Range;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{Color, ContentStyle};

use super::{
    count_as_string,
    layout::{Line, LineBuilder},
};

#[derive(Default, PartialEq, Debug)]
pub enum Status {
    #[default]
    Skipped,
    Updated,
    UpdatedCursor,
    NotMapped,
    PasteInvoked,
    Copy(String),
    Cut(String),
}

impl Status {
    /// includes cursor updates
    pub fn is_updated(&self) -> bool {
        match self {
            Self::Updated | Self::UpdatedCursor | Self::Cut(..) => true,
            Self::Skipped | Self::NotMapped | Self::Copy(..) | Self::PasteInvoked => false,
        }
    }

    pub fn is_text_updated(&self) -> bool {
        match self {
            Self::Updated | Self::Cut(..) => true,
            Self::UpdatedCursor | Self::Skipped | Self::NotMapped | Self::Copy(..) | Self::PasteInvoked => false,
        }
    }

    pub fn is_mapped(&self) -> bool {
        !matches!(self, Self::NotMapped)
    }
}

#[derive(Default)]
pub struct TextField {
    pub text: String,
    char: usize,
    select: Option<(usize, usize)>,
}

impl TextField {
    pub fn new(text: String) -> Self {
        Self { char: text.len(), text, select: None }
    }

    pub fn text_set(&mut self, text: String) {
        self.select = None;
        self.text = text;
        self.char = self.text.len();
    }

    #[allow(dead_code)]
    pub fn text_take(&mut self) -> String {
        self.char = 0;
        self.select = None;
        std::mem::take(&mut self.text)
    }

    #[allow(dead_code)]
    pub fn text_get_token_at_cursor(&self) -> Option<&str> {
        let token_range = arg_range_at(&self.text, self.char);
        self.text.get(token_range)
    }

    #[allow(dead_code)]
    pub fn text_replace_token(&mut self, new: &str) {
        let token_range = arg_range_at(&self.text, self.char);
        self.char = new.len() + token_range.start;
        self.select = None;
        self.text.replace_range(token_range, new);
    }

    /// returns blockless paragraph widget " >> inner text"
    pub fn widget(&self, line: Line, backend: &mut impl Backend) {
        let mut builder = line.unsafe_builder(backend);
        builder.push(" >> ");
        self.insert_formatted_text(builder);
    }

    /// returns blockless paragraph widget "99+ >> inner text"
    #[allow(dead_code)]
    pub fn widget_with_count(&self, line: Line, count: usize, backend: &mut impl Backend) {
        let mut builder = line.unsafe_builder(backend);
        builder.push(count_as_string(count).as_str());
        builder.push(" >> ");
        self.insert_formatted_text(builder);
    }

    pub fn insert_formatted_text<B: Backend>(&self, line_builder: LineBuilder<B>) {
        match self.select.as_ref().map(|(f, t)| if f > t { (*t, *f) } else { (*f, *t) }) {
            Some((from, to)) => self.text_cursor_select(from, to, line_builder),
            None => self.text_cursor(line_builder),
        };
    }

    fn text_cursor<B: Backend>(&self, mut builder: LineBuilder<B>) {
        if self.char == self.text.len() {
            builder.push(&self.text);
            builder.push_styled(" ", ContentStyle::reversed());
        } else {
            builder.push(self.text[..self.char].as_ref());
            builder.push_styled(self.text[self.char..=self.char].as_ref(), ContentStyle::reversed());
            builder.push(self.text[self.char + 1..].as_ref());
        };
    }

    fn text_cursor_select<B: Backend>(&self, from: usize, to: usize, mut builder: LineBuilder<B>) {
        builder.push(self.text[..from].as_ref());
        if from == self.char {
            builder.push_styled(self.text[self.char..=self.char].as_ref(), ContentStyle::reversed());
            builder.push_styled(self.text[from + 1..to].as_ref(), ContentStyle::bg(Color::Rgb { r: 72, g: 72, b: 72 }));
            builder.push(self.text[to..].as_ref());
        } else if self.char == self.text.len() {
            builder.push_styled(self.text[from..to].as_ref(), ContentStyle::bg(Color::Rgb { r: 72, g: 72, b: 72 }));
            builder.push(self.text[to..].as_ref());
            builder.push_styled(" ", ContentStyle::reversed());
        } else {
            builder.push_styled(self.text[from..to].as_ref(), ContentStyle::bg(Color::Rgb { r: 72, g: 72, b: 72 }));
            builder.push_styled(self.text[to..=to].as_ref(), ContentStyle::reversed());
            builder.push(self.text[to + 1..].as_ref());
        }
    }

    pub fn paste_passthrough(&mut self, clip: String) -> Status {
        if !clip.contains('\n') {
            self.take_selected();
            self.text.insert_str(self.char, clip.as_str());
            self.char += clip.len();
            return Status::Updated;
        };
        Status::default()
    }

    pub fn map(&mut self, key: &KeyEvent) -> Status {
        match key.code {
            KeyCode::Char('c' | 'C')
                if key.modifiers == KeyModifiers::CONTROL
                    || key.modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT =>
            {
                if let Some(clip) = self.get_selected() {
                    return Status::Copy(clip);
                };
                Status::default()
            }
            KeyCode::Char('x' | 'X') if key.modifiers == KeyModifiers::CONTROL => {
                if let Some(clip) = self.take_selected() {
                    return Status::Cut(clip);
                };
                Status::default()
            }
            KeyCode::Char('x' | 'X') if key.modifiers == KeyModifiers::CONTROL => Status::PasteInvoked,
            KeyCode::Char(ch) => {
                self.take_selected();
                self.text.insert(self.char, ch);
                self.char += 1;
                Status::Updated
            }
            KeyCode::Delete => {
                if self.take_selected().is_some() {
                    return Status::Updated;
                };
                if self.char < self.text.len() && !self.text.is_empty() {
                    self.text.remove(self.char);
                    return Status::Updated;
                }
                Status::Skipped
            }
            KeyCode::Backspace => {
                if self.take_selected().is_some() {
                    return Status::Updated;
                };
                if self.char > 0 && !self.text.is_empty() {
                    self.char -= 1;
                    self.text.remove(self.char);
                    return Status::Updated;
                };
                Status::Skipped
            }
            KeyCode::End => {
                self.char = self.text.len().saturating_sub(1);
                Status::UpdatedCursor
            }
            KeyCode::Left => {
                self.move_left(key.modifiers);
                Status::UpdatedCursor
            }
            KeyCode::Right => {
                self.move_right(key.modifiers);
                Status::UpdatedCursor
            }
            _ => Status::NotMapped,
        }
    }

    fn move_left(&mut self, mods: KeyModifiers) {
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
    }

    fn move_right(&mut self, mods: KeyModifiers) {
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
    }

    pub fn copy(&mut self) -> Option<String> {
        self.get_selected()
    }

    pub fn cut(&mut self) -> Option<String> {
        self.take_selected()
    }

    /// returns false if clip contains new line
    pub fn try_paste(&mut self, clip: String) -> bool {
        if clip.contains('\n') {
            return false;
        }
        self.take_selected();
        self.text.insert_str(self.char, clip.as_str());
        self.char += clip.len();
        true
    }

    pub fn push_char(&mut self, ch: char) {
        self.take_selected();
        self.text.insert(self.char, ch);
        self.char += 1;
    }

    pub fn del(&mut self) {
        if self.take_selected().is_some() {
            return;
        }
        if self.char < self.text.len() && !self.text.is_empty() {
            self.text.remove(self.char);
        };
    }

    pub fn backspace(&mut self) {
        if self.take_selected().is_some() {
            return;
        }
        if self.char > 0 && !self.text.is_empty() {
            self.char -= 1;
            self.text.remove(self.char);
        };
    }

    pub fn go_to_end_of_line(&mut self) {
        self.char = self.text.len().saturating_sub(1);
    }

    pub fn go_left(&mut self) {
        self.char = self.char.saturating_sub(1);
        self.select = None;
    }

    pub fn select_left(&mut self) {
        self.init_select();
        self.char = self.char.saturating_sub(1);
        self.push_select();
    }

    pub fn jump_left(&mut self) {
        self.select = None;
        self.jump_right_move();
    }

    pub fn select_jump_left(&mut self) {
        self.init_select();
        self.jump_left_move();
        self.push_select();
    }

    pub fn go_right(&mut self) {
        self.char = std::cmp::min(self.text.len(), self.char + 1);
        self.select = None;
    }

    pub fn select_right(&mut self) {
        self.init_select();
        self.char = std::cmp::min(self.text.len(), self.char + 1);
        self.push_select();
    }

    pub fn jump_right(&mut self) {
        self.select = None;
        self.jump_left_move();
    }

    pub fn select_jump_right(&mut self) {
        self.init_select();
        self.jump_right_move();
        self.push_select();
    }

    fn jump_left_move(&mut self) {
        while self.char > 0 {
            let next_idx = self.char - 1;
            if matches!(self.text.chars().nth(next_idx), Some(ch) if !ch.is_alphabetic() && !ch.is_numeric()) {
                break;
            };
            self.char = next_idx;
        }
    }

    fn jump_right_move(&mut self) {
        while self.text.len() > self.char {
            self.char += 1;
            if matches!(self.text.chars().nth(self.char), Some(ch) if !ch.is_alphabetic() && !ch.is_numeric()) {
                break;
            }
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

#[cfg(test)]
mod test {
    use crate::text_field::Status;

    use super::TextField;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_setting() {
        let mut field = TextField::default();
        field.text_set("12345".to_owned());
        assert_eq!(&field.text, "12345");
        assert_eq!(field.char, 5);
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT));
        assert!(field.select.is_some());
        assert_eq!(field.char, 4);
        assert_eq!(&field.text_take(), "12345");
        assert_eq!(field.char, 0);
        assert_eq!(&field.text, "");
        assert!(field.select.is_none());
    }

    #[test]
    fn test_move() {
        let mut field = TextField::default();
        assert_eq!(field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty())), Status::UpdatedCursor);
        assert!(field.char == 0);
        field.text_set("12".to_owned());
        assert_eq!(field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL)), Status::UpdatedCursor);
        assert_eq!(field.char, 2);
        assert_eq!(field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL)), Status::UpdatedCursor);
        assert_eq!(field.char, 0);
        assert_eq!(field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty())), Status::UpdatedCursor);
        assert_eq!(field.char, 1);
        assert_eq!(field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::empty())), Status::UpdatedCursor);
        assert_eq!(field.char, 0);
    }

    #[test]
    fn test_select() {
        let mut field = TextField::default();
        field.text_set("a3cde".to_owned());
        field.char = 0;
        assert_eq!(
            field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::SHIFT,)),
            Status::UpdatedCursor
        );
        assert_eq!(field.select, Some((0, 5)));
        assert_eq!(field.char, 5);
        assert_eq!(field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty())), Status::UpdatedCursor);
        assert!(field.select.is_none());
        assert_eq!(field.char, 5);
        assert_eq!(field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT)), Status::UpdatedCursor);
        assert_eq!(field.select, Some((5, 4)));
        assert_eq!(
            field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::CONTROL,)),
            Status::UpdatedCursor
        );
        assert_eq!(field.select, Some((5, 0)));
        assert_eq!(field.map(&KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty())), Status::Updated);
        assert_eq!(&field.text, "");
    }
}
