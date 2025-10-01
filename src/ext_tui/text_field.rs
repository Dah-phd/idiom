use crate::{
    configs::EditorAction,
    ext_tui::{CrossTerm, StyleExt},
    global_state::Clipboard,
};
use core::{ops::Range, fmt::Display};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{Color, ContentStyle};
use idiom_tui::{
    count_as_string,
    layout::{Line, LineBuilder},
};

#[derive(Default, PartialEq, Debug, Clone)]
pub struct TextField<T: Default + Clone> {
    text: String,
    char: usize,
    select: Option<(usize, usize)>,
    select_style: ContentStyle,
    on_text_update: Option<T>,
}

impl<T: Default + Clone> TextField<T> {
    pub fn new(text: String, on_text_update: Option<T>) -> Self {
        Self {
            char: text.len(),
            text,
            select: None,
            on_text_update,
            select_style: ContentStyle::bg(Color::Rgb { r: 72, g: 72, b: 72 }),
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.text.len()
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
    pub fn widget(&self, line: Line, backend: &mut CrossTerm) {
        let mut builder = line.unsafe_builder(backend);
        builder.push(" >> ");
        self.insert_formatted_text(builder);
    }

    /// returns blockless paragraph widget "99+ >> inner text"
    #[allow(dead_code)]
    pub fn widget_with_count(&self, line: Line, count: usize, backend: &mut CrossTerm) {
        let mut builder = line.unsafe_builder(backend);
        builder.push(count_as_string(count).as_str());
        builder.push(" >> ");
        self.insert_formatted_text(builder);
    }

    pub fn insert_formatted_text(&self, line_builder: LineBuilder<CrossTerm>) {
        match self.select.as_ref().map(|(f, t)| if f > t { (*t, *f) } else { (*f, *t) }) {
            Some((from, to)) if from != to => self.text_cursor_select(from, to, line_builder),
            _ => self.text_cursor(line_builder),
        };
    }

    fn text_cursor(&self, mut builder: LineBuilder<CrossTerm>) {
        match self.get_cursor_range() {
            Some(cursor) => {
                let Range { start, end } = cursor;
                builder.push(&self.text[..start]);
                builder.push_styled(&self.text[cursor], ContentStyle::reversed());
                builder.push(&self.text[end..]);
            }
            None => {
                builder.push(&self.text);
                builder.push_styled(" ", ContentStyle::reversed());
            }
        };
    }

    fn text_cursor_select(&self, from: usize, to: usize, mut builder: LineBuilder<CrossTerm>) {
        builder.push(self.text[..from].as_ref());
        match self.get_cursor_range() {
            Some(cursor) => {
                let Range { start, end } = cursor;
                if from == start {
                    builder.push_styled(&self.text[cursor], ContentStyle::reversed());
                    builder.push_styled(&self.text[end..to], self.select_style);
                    builder.push(&self.text[to..]);
                } else {
                    builder.push_styled(&self.text[from..start], self.select_style);
                    builder.push_styled(&self.text[cursor], ContentStyle::reversed());
                    builder.push(&self.text[end..]);
                }
            }
            None => {
                builder.push_styled(self.text[from..to].as_ref(), self.select_style);
                builder.push(self.text[to..].as_ref());
                builder.push_styled(" ", ContentStyle::reversed());
            }
        }
    }

    pub fn paste_passthrough(&mut self, clip: String) -> T {
        if !clip.contains('\n') {
            self.take_selected();
            self.text.insert_str(self.char, clip.as_str());
            self.char += clip.len();
            return self.on_text_update.clone().unwrap_or_default();
        };
        T::default()
    }

    pub fn map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> Option<T> {
        match key.code {
            KeyCode::Char('c' | 'C')
                if key.modifiers == KeyModifiers::CONTROL
                    || key.modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT =>
            {
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
            KeyCode::Char('a' | 'A') if key.modifiers == KeyModifiers::CONTROL => {
                self.select_all();
                Some(T::default())
            }
            KeyCode::Char(ch) => {
                self.take_selected();
                self.text.insert(self.char, ch);
                self.char += ch.len_utf8();
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
                    self.prev_char();
                    self.text.remove(self.char);
                };
                Some(self.on_text_update.clone().unwrap_or_default())
            }
            KeyCode::End => {
                self.select = None;
                self.char = self.text.len();
                Some(T::default())
            }
            KeyCode::Left => self.move_left(key.modifiers),
            KeyCode::Right => self.move_right(key.modifiers),
            _ => None,
        }
    }

    pub fn map_actions(&mut self, action: EditorAction, clipboard: &mut Clipboard) -> Option<T> {
        match action {
            EditorAction::Copy => {
                if let Some(clip) = self.get_selected() {
                    clipboard.push(clip);
                };
                Some(T::default())
            }
            EditorAction::Cut => {
                if let Some(clip) = self.take_selected() {
                    clipboard.push(clip);
                    return Some(self.on_text_update.clone().unwrap_or_default());
                };
                Some(T::default())
            }
            EditorAction::Paste => {
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
            EditorAction::Char(ch) => {
                self.take_selected();
                self.text.insert(self.char, ch);
                self.char += ch.len_utf8();
                Some(self.on_text_update.clone().unwrap_or_default())
            }
            EditorAction::Delete => {
                if self.take_selected().is_some() {
                } else if self.char < self.text.len() && !self.text.is_empty() {
                    self.text.remove(self.char);
                };
                Some(self.on_text_update.clone().unwrap_or_default())
            }
            EditorAction::Backspace => {
                if self.take_selected().is_some() {
                } else if self.char > 0 && !self.text.is_empty() {
                    self.prev_char();
                    self.text.remove(self.char);
                };
                Some(self.on_text_update.clone().unwrap_or_default())
            }
            EditorAction::EndOfLine | EditorAction::EndOfFile => {
                self.select = None;
                self.char = self.text.len();
                Some(T::default())
            }
            EditorAction::Left => {
                self.prev_char();
                self.select = None;
                Some(T::default())
            }
            EditorAction::SelectLeft => {
                self.init_select();
                self.prev_char();
                self.push_select();
                Some(T::default())
            }
            EditorAction::JumpLeft => {
                self.select = None;
                self.jump_left();
                Some(T::default())
            }
            EditorAction::JumpLeftSelect => {
                self.init_select();
                self.jump_left();
                self.push_select();
                Some(T::default())
            }
            EditorAction::Right => {
                self.next_char();
                self.select = None;
                Some(T::default())
            }
            EditorAction::SelectRight => {
                self.init_select();
                self.next_char();
                self.push_select();
                Some(T::default())
            }
            EditorAction::JumpRight => {
                self.select = None;
                self.jump_right();
                Some(T::default())
            }
            EditorAction::JumpRightSelect => {
                self.init_select();
                self.jump_right();
                self.push_select();
                Some(T::default())
            }
            EditorAction::SelectAll => {
                self.select_all();
                Some(T::default())
            }
            _ => None,
        }
    }

    pub fn select_all(&mut self) {
        self.select = Some((0, self.text.len()));
        self.char = self.text.len();
    }

    pub fn select_token(&mut self) {
        let range = arg_range_at(&self.text, self.char);
        if !range.is_empty() {
            self.select = Some((range.start, range.end));
            self.char = range.end;
        }
    }

    /// in most cases there is offset that needs to be handled outside
    /// for widget the value is 4
    /// for counted widget the value is 7
    pub fn click_char(&mut self, rel_char: usize) {
        if self.char == rel_char {
            self.select_token();
            return;
        }
        self.select.take();
        self.char = std::cmp::min(rel_char, self.text.len());
    }

    fn get_cursor_range(&self) -> Option<Range<usize>> {
        let cursor_char = self.text[self.char..].chars().next()?;
        Some(self.char..self.char + cursor_char.len_utf8())
    }

    fn next_char(&mut self) {
        self.char += self.text[self.char..].chars().next().map(|ch| ch.len_utf8()).unwrap_or_default();
    }

    fn prev_char(&mut self) {
        self.char -= self.text[..self.char].chars().next_back().map(|ch| ch.len_utf8()).unwrap_or_default();
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

    fn move_left(&mut self, mods: KeyModifiers) -> Option<T> {
        let should_select = mods.contains(KeyModifiers::SHIFT);
        if should_select {
            self.init_select();
        } else {
            self.select = None;
        };
        self.prev_char();
        if mods.contains(KeyModifiers::CONTROL) {
            self.jump_left();
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
        self.next_char();
        if mods.contains(KeyModifiers::CONTROL) {
            self.jump_right();
        };
        if should_select {
            self.push_select();
        };
        Some(T::default())
    }

    fn jump_left(&mut self) {
        for (idx, ch) in self.text[..self.char].char_indices().rev() {
            if !ch.is_alphabetic() && !ch.is_numeric() {
                return;
            }
            self.char = idx;
        }
    }

    fn jump_right(&mut self) {
        // jump
        for (idx, ch) in self.text[self.char..].char_indices() {
            if !ch.is_alphabetic() && !ch.is_numeric() {
                self.char += idx;
                return;
            }
        }
        self.char = self.text.len();
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

impl <T: Clone + Default>Display for TextField<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.text.fmt(f)
    }
}

impl TextField<()> {
    pub fn basic(text: String) -> Self {
        Self { char: text.len(), text, ..Default::default() }
    }
}

#[cfg(test)]
pub mod test {
    use super::TextField;
    use crate::ext_tui::{CrossTerm, StyleExt};
    use crate::global_state::Clipboard;
    use crossterm::{
        event::{KeyCode, KeyEvent, KeyModifiers},
        style::{Color, ContentStyle},
    };
    use idiom_tui::{layout::Line, Backend};

    pub fn pull_select<T: Clone + Default>(text_field: &TextField<T>) -> Option<(usize, usize)> {
        text_field.select
    }

    pub fn pull_char<T: Clone + Default>(text_field: &TextField<T>) -> usize {
        text_field.char
    }

    #[test]
    fn render_non_ascii() {
        let mut field = TextField::new("a a🦀🦀ssd asd 🦀s".to_owned(), Some(true));
        let mut backend = CrossTerm::init();
        let line = Line { row: 0, col: 1, width: 50 };
        field.widget(line, &mut backend);
        let mut cliptboard = Clipboard::default();

        assert_eq!(
            backend.drain(),
            &[
                (ContentStyle::default(), "<<go to row: 0 col: 1>>".to_owned()),
                (ContentStyle::default(), " >> ".to_owned()),
                (ContentStyle::default(), "a a🦀🦀ssd asd 🦀s".to_owned()),
                (ContentStyle::reversed(), " ".to_owned()),
                (ContentStyle::default(), "<<padding: 27>>".to_owned()),
            ]
        );

        field.char = 0;
        assert!(!field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty()), &mut cliptboard).unwrap());
        assert!(!field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty()), &mut cliptboard).unwrap());
        assert!(!field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty()), &mut cliptboard).unwrap());

        let line = Line { row: 0, col: 1, width: 50 };
        field.widget(line, &mut backend);
        assert_eq!(
            backend.drain(),
            &[
                (ContentStyle::default(), "<<go to row: 0 col: 1>>".to_owned()),
                (ContentStyle::default(), " >> ".to_owned()),
                (ContentStyle::default(), "a a".to_owned()),
                (ContentStyle::reversed(), "🦀".to_owned()),
                (ContentStyle::default(), "🦀ssd asd 🦀s".to_owned()),
                (ContentStyle::default(), "<<padding: 28>>".to_owned()),
            ]
        );

        assert!(!field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty()), &mut cliptboard).unwrap());
        assert!(!field
            .map(&KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::SHIFT), &mut cliptboard)
            .unwrap());

        let line = Line { row: 0, col: 1, width: 50 };
        field.widget(line, &mut backend);
        assert_eq!(
            backend.drain(),
            &[
                (ContentStyle::default(), "<<go to row: 0 col: 1>>".to_owned()),
                (ContentStyle::default(), " >> ".to_owned()),
                (ContentStyle::default(), "a a🦀".to_owned()),
                (ContentStyle::bg(Color::Rgb { r: 72, g: 72, b: 72 }), "🦀ssd".to_owned()),
                (ContentStyle::reversed(), " ".to_owned()),
                (ContentStyle::default(), "asd 🦀s".to_owned()),
                (ContentStyle::default(), "<<padding: 28>>".to_owned()),
            ]
        );
    }

    #[test]
    fn render_with_number() {
        let field = TextField::new("some text".to_owned(), Some(true));
        let mut backend = CrossTerm::init();
        let line = Line { row: 0, col: 1, width: 50 };

        field.widget_with_count(line, 3, &mut backend);

        assert_eq!(
            backend.drain(),
            &[
                (ContentStyle::default(), "<<go to row: 0 col: 1>>".to_owned()),
                (ContentStyle::default(), "  3".to_owned()),
                (ContentStyle::default(), " >> ".to_owned()),
                (ContentStyle::default(), "some text".to_owned()),
                (ContentStyle::reversed(), " ".to_owned()),
                (ContentStyle::default(), "<<padding: 33>>".to_owned()),
            ]
        );
    }

    #[test]
    fn setting() {
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
    fn setting_non_ascii() {
        let mut field: TextField<()> = TextField::default();
        field.text_set("12🦀45".to_owned());
        assert_eq!(&field.text, "12🦀45");
        assert_eq!(field.char, 8);
        let mut clip = Clipboard::default();
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT), &mut clip);
        assert_eq!(field.char, 7);
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT), &mut clip);
        assert_eq!(field.char, 6);
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT), &mut clip);
        assert!(field.select.is_some());
        assert_eq!(field.char, 2);
        assert_eq!(&field.text_take(), "12🦀45");
        assert_eq!(field.char, 0);
        assert_eq!(&field.text, "");
        assert!(field.select.is_none());
    }

    #[test]
    fn move_presses() {
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
    fn move_presses_non_ascii() {
        let mut field: TextField<()> = TextField::default();
        let mut clip = Clipboard::default();
        field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty()), &mut clip);
        assert!(field.char == 0);
        field.text_set("🦀1".to_owned());
        field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL), &mut clip);
        assert_eq!(field.char, 5);
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL), &mut clip);
        assert_eq!(field.char, 4);
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::empty()), &mut clip);
        assert_eq!(field.char, 0);
        field.map(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty()), &mut clip);
        assert_eq!(field.char, 4);
        field.map(&KeyEvent::new(KeyCode::Left, KeyModifiers::empty()), &mut clip);
        assert_eq!(field.char, 0);
    }

    #[test]
    fn select() {
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

    #[test]
    fn select_all() {
        let mut tf = TextField::basic("1234".to_string());
        tf.select_all();
        assert_eq!(tf.select, Some((0, 4)));
        assert_eq!("1234", tf.take_selected().unwrap());
    }

    #[test]
    fn select_token_and_char_set() {
        let mut tf = TextField::basic("asd baba".to_string());
        assert_eq!(tf.char, 8);
        tf.click_char(8);
        assert_eq!(tf.select, Some((4, 8)));
        assert_eq!(tf.char, 8);
        tf.click_char(2);
        assert_eq!(tf.select, None);
        assert_eq!(tf.char, 2);
        tf.click_char(2);
        assert_eq!(tf.select, Some((0, 3)));
        assert_eq!(tf.char, 3);
        tf.click_char(6);
        assert_eq!(tf.char, 6);
        tf.select_token();
        assert_eq!(tf.select, Some((4, 8)));
        assert_eq!(tf.char, 8);
    }
}
