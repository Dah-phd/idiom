use super::select::CursorPosition;
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
        Self {
            from_line,
            cursor,
            old_content,
        }
    }

    fn from_cursor(cursor: CursorPosition, old_content: Vec<String>) -> Self {
        Self {
            from_line: cursor.line,
            cursor,
            old_content,
        }
    }

    fn collect(self, new_cursor: CursorPosition, new: Vec<String>) -> Action {
        Action {
            from_line: self.from_line,
            old_cursor: self.cursor,
            new_cursor,
            old: self.old_content,
            new,
        }
    }
}

#[derive(Debug)]
pub struct ActionLogger {
    replace_builder: Option<ReplaceBuilder>,
    buffer: Option<Action>,
    pub done: Vec<Action>,
    pub undone: Vec<Action>,
    clock: Instant,
}

impl Default for ActionLogger {
    fn default() -> Self {
        Self {
            replace_builder: Option::default(),
            buffer: Option::default(),
            done: Vec::default(),
            undone: Vec::default(),
            clock: Instant::now(),
        }
    }
}

impl ActionLogger {
    pub fn tick(&mut self) {
        if self.buffer.is_some() {
            if self.clock.elapsed() >= TICK {
                self.push_buffer();
            }
            self.clock = Instant::now();
        }
    }

    pub fn init_replace_from_select(&mut self, from: &CursorPosition, to: &CursorPosition, content: &[String]) {
        self.undone.clear();
        self.push_buffer();
        self.replace_builder = Some(ReplaceBuilder::from_cursor(
            *from,
            content[from.line..=to.line].to_owned(),
        ))
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
            self.done.push(builder.collect(new_cursor, new.into()))
        }
    }

    pub fn backspace(&mut self, cursor: &CursorPosition, old_line: &str) {
        self.undone.clear();
        self.tick();
        if let Some(action) = &mut self.buffer {
            if &action.new_cursor == cursor {
                action.new_cursor.char -= 1;
                action.new[0].remove(action.new_cursor.char);
                return;
            }
            self.push_buffer();
        }
        let mut action = Action::basic(*cursor, old_line);
        action.new_cursor.char -= 1;
        action.new[0].remove(action.new_cursor.char);
        self.set_buffer(action);
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
        if let Some(buffer) = self.buffer.take() {
            self.done.push(buffer)
        }
    }

    pub fn undo(&mut self, content: &mut Vec<String>) -> Option<CursorPosition> {
        self.push_buffer();
        let action = self.done.pop()?;
        let old_cursor = action.restore(content);
        self.undone.push(action.reverse());
        Some(old_cursor)
    }

    pub fn redo(&mut self, content: &mut Vec<String>) -> Option<CursorPosition> {
        self.push_buffer();
        let action = self.undone.pop()?;
        let old_cursor = action.restore(content);
        self.done.push(action.reverse());
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
}
