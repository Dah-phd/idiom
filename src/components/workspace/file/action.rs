use super::{
    cursor::CursorPosition,
    utils::{apply_and_rev_edit, into_content_event},
};
use lsp_types::{Position, Range, TextDocumentContentChangeEvent, TextEdit};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(150);

#[derive(Debug)]
struct ReplaceBuilder {
    from_line: usize,
    cursor: CursorPosition,
    old_content: Vec<String>,
}

impl ReplaceBuilder {
    fn new(from_line: usize, cursor: CursorPosition, old_content: Vec<String>) -> Self {
        Self { from_line, cursor, old_content }
    }

    fn from_cursor(cursor: CursorPosition, old_content: Vec<String>) -> Self {
        Self { from_line: cursor.line, cursor, old_content }
    }

    fn collect(self, new_cursor: CursorPosition, new: Vec<String>) -> Action {
        Action { from_line: self.from_line, old_cursor: self.cursor, new_cursor, old: self.old_content, new }
    }

    fn test() {
        println!("uraa");
        println!("hello");
        println!("world!");
        println!("test");
    }
}

#[derive(Debug)]
pub struct ActionLogger {
    replace_builder: Option<ReplaceBuilder>,
    buffer: Option<Action>,
    version: i32,
    text_edits: Vec<TextDocumentContentChangeEvent>,
    pub done: Vec<Action>,
    pub undone: Vec<Action>,
    clock: Instant,
}

impl Default for ActionLogger {
    fn default() -> Self {
        Self {
            replace_builder: None,
            buffer: None,
            done: Vec::default(),
            text_edits: Vec::default(),
            version: 0,
            undone: Vec::default(),
            clock: Instant::now(),
        }
    }
}

impl ActionLogger {
    pub fn tick(&mut self) {
        if self.buffer.is_some() {
            self.push_buffer();
            self.clock = Instant::now();
        }
    }

    pub fn init_replace_from_select(&mut self, from: &CursorPosition, to: &CursorPosition, content: &[String]) {
        self.undone.clear();
        self.push_buffer();
        self.replace_builder = Some(ReplaceBuilder::from_cursor(*from, content[from.line..=to.line].to_owned()))
    }

    pub fn init_repalce_from_line(&mut self, from_line: usize, cursor: CursorPosition, old_content: &[String]) {
        self.undone.clear();
        self.push_buffer();
        self.replace_builder = Some(ReplaceBuilder::new(from_line, cursor, old_content.into()))
    }

    pub fn init_replace(&mut self, cursor: CursorPosition, old_content: &[String]) {
        self.undone.clear();
        self.push_buffer();
        self.replace_builder = Some(ReplaceBuilder::from_cursor(cursor, old_content.to_owned()))
    }

    pub fn finish_replace(&mut self, new_cursor: CursorPosition, new: &[String]) {
        if let Some(builder) = self.replace_builder.take() {
            self.push_done(builder.collect(new_cursor, new.into()));
        }
    }

    pub fn prep_buffer(&mut self, cursor: &CursorPosition, current_line: &str) {
        self.undone.clear();
        self.tick();
        if let Some(action) = &mut self.buffer {
            if &action.new_cursor == cursor {
                return;
            }
            self.push_buffer();
        }
        self.set_buffer(Action::basic(*cursor, current_line))
    }

    pub fn backspace(&mut self, cursor: &CursorPosition) {
        if let Some(action) = &mut self.buffer {
            action.new[0].replace_range(cursor.char..action.new_cursor.char, "");
            action.new_cursor = *cursor;
        }
    }

    pub fn buffer_str(&mut self, string: &str, new_cursor: CursorPosition) {
        if let Some(action) = &mut self.buffer {
            action.new[0].insert_str(action.new_cursor.char, string);
            action.new_cursor = new_cursor;
        }
    }

    pub fn del(&mut self, cursor: &CursorPosition, old_line: &str) {
        self.undone.clear();
        self.tick();
        if let Some(action) = &mut self.buffer {
            if &action.new_cursor == cursor {
                action.new[0].remove(action.new_cursor.char);
                return;
            }
            self.push_buffer();
        }
        let mut action = Action::basic(*cursor, old_line);
        action.new[0].remove(action.new_cursor.char);
        self.set_buffer(action);
    }

    pub fn push_char(&mut self, cursor: &CursorPosition, old_line: &str, ch: char) {
        self.buffer_inser(cursor, old_line, ch, 1);
    }

    pub fn inser_char(&mut self, cursor: &CursorPosition, old_line: &str, ch: char) {
        self.buffer_inser(cursor, old_line, ch, 0)
    }

    fn buffer_inser(&mut self, cursor: &CursorPosition, old_line: &str, ch: char, cursor_bump: usize) {
        self.undone.clear();
        self.tick();
        if let Some(action) = &mut self.buffer {
            if &action.new_cursor == cursor {
                action.new[0].insert(action.new_cursor.char, ch);
                action.new_cursor.char += cursor_bump;
                return;
            }
            self.push_buffer()
        }
        let mut action = Action::basic(*cursor, old_line);
        action.new[0].insert(action.new_cursor.char, ch);
        action.new_cursor.char += cursor_bump;
        self.set_buffer(action);
    }

    fn set_buffer(&mut self, buffer: Action) {
        self.buffer = Some(buffer);
        self.clock = Instant::now();
    }

    fn push_buffer(&mut self) {
        if self.clock.elapsed() >= TICK {
            if let Some(buffer) = self.buffer.take() {
                self.push_done(buffer)
            }
        }
    }

    fn push_done(&mut self, action: Action) {
        self.text_edits.push(action.get_text_edit());
        self.done.push(action);
    }

    fn push_undone(&mut self, action: Action) {
        self.text_edits.push(action.get_text_edit());
        self.undone.push(action);
    }

    pub fn get_text_edits(&mut self) -> Option<(i32, Vec<TextDocumentContentChangeEvent>)> {
        self.push_buffer();
        if self.text_edits.is_empty() {
            None
        } else {
            self.version += 1;
            Some((self.version, self.text_edits.drain(..).collect()))
        }
    }

    pub fn undo(&mut self, content: &mut Vec<String>) -> Option<CursorPosition> {
        self.push_buffer();
        let action = self.done.pop()?;
        let old_cursor = action.restore(content);
        self.push_undone(action.reverse());
        Some(old_cursor)
    }

    pub fn redo(&mut self, content: &mut Vec<String>) -> Option<CursorPosition> {
        self.push_buffer();
        let action = self.undone.pop()?;
        let old_cursor = action.restore(content);
        self.push_done(action.reverse());
        Some(old_cursor)
    }
}

#[derive(Debug)]
pub struct Action {
    from_line: usize,
    old_cursor: CursorPosition,
    new_cursor: CursorPosition,
    old: Vec<String>,
    new: Vec<String>,
}

impl Action {
    fn basic(cursor: CursorPosition, init_line: &str) -> Self {
        Self {
            from_line: cursor.line,
            old_cursor: cursor,
            new_cursor: cursor,
            old: vec![String::from(init_line)],
            new: vec![String::from(init_line)],
        }
    }
    fn reverse(self) -> Self {
        Self {
            from_line: self.from_line,
            old_cursor: self.new_cursor,
            new_cursor: self.old_cursor,
            old: self.new,
            new: self.old,
        }
    }

    fn restore(&self, content: &mut Vec<String>) -> CursorPosition {
        let mut lines_to_remove = self.new.len();
        while lines_to_remove != 0 {
            content.remove(self.from_line);
            lines_to_remove -= 1;
        }
        for line in self.old.iter().rev() {
            content.insert(self.from_line, line.into())
        }
        self.old_cursor
    }

    fn get_text_edit(&self) -> TextDocumentContentChangeEvent {
        let start = Position::new(self.old_cursor.line as u32, 0);
        let end = Position::new(start.line + self.old.len() as u32, 0);
        let range = Range::new(start, end);
        let mut text = self.new.join("\n");
        if !self.new.is_empty() {
            text.push('\n');
        }
        TextDocumentContentChangeEvent { range: Some(range), range_length: None, text }
    }
}
