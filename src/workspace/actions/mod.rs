mod action_buffer;
mod edits;

use crate::configs::EditorConfigs;
use crate::syntax::Lexer;
use crate::workspace::{
    cursor::{Cursor, CursorPosition},
    utils::{get_closing_char, is_closing_repeat},
};
use action_buffer::ActionBuffer;
pub use edits::{Edit, EditBuilder, EditMetaData};
use lsp_types::{TextDocumentContentChangeEvent, TextEdit};

#[derive(Debug, Default)]
pub struct Actions {
    pub cfg: EditorConfigs,
    version: i32,
    done: Vec<EditType>,
    undone: Vec<EditType>,
    events: Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
    buffer: ActionBuffer,
}

impl Actions {
    pub fn new(cfg: EditorConfigs) -> Self {
        Self { cfg, events: Vec::with_capacity(2), ..Default::default() }
    }

    pub fn swap_up(&mut self, cursor: &mut Cursor, content: &mut [String]) {
        if cursor.line == 0 {
            return;
        }
        cursor.select_drop();
        self.push_buffer();
        cursor.line -= 1;
        let (top, _, action) = Edit::swap_down(cursor.line, &self.cfg, content);
        cursor.char = top.offset(cursor.char);
        self.push_done(action);
    }

    pub fn swap_down(&mut self, cursor: &mut Cursor, content: &mut Vec<String>) {
        if content.is_empty() || content.len() - 1 <= cursor.line {
            return;
        }
        cursor.select_drop();
        self.push_buffer();
        let (_, bot, action) = Edit::swap_down(cursor.line, &self.cfg, content);
        cursor.line += 1;
        cursor.char = bot.offset(cursor.char);
        self.push_done(action);
    }

    pub fn replace_token(&mut self, new: String, cursor: &mut Cursor, content: &mut [String]) {
        self.push_buffer();
        let action = Edit::replace_token(cursor.line, cursor.char, new, content);
        cursor.char = action.reverse_text_edit.range.end.character as usize;
        self.push_done(action);
    }

    pub fn replace_select(
        &mut self,
        from: CursorPosition,
        to: CursorPosition,
        clip: impl Into<String>,
        cursor: &mut Cursor,
        content: &mut Vec<String>,
    ) {
        self.push_buffer();
        cursor.select_drop();
        let action = Edit::replace_select(from, to, clip.into(), content);
        cursor.set_position(action.end_position());
        self.push_done(action);
    }

    pub fn mass_replace(
        &mut self,
        cursor: &mut Cursor,
        mut ranges: Vec<(CursorPosition, CursorPosition)>,
        clip: String,
        content: &mut Vec<String>,
    ) {
        self.push_buffer();
        cursor.select_drop();
        let actions: Vec<_> =
            ranges.drain(..).map(|(from, to)| Edit::replace_select(from, to, clip.to_owned(), content)).collect();
        if let Some(last) = actions.last() {
            cursor.set_position(last.end_position());
        }
        self.push_done(actions);
    }

    pub fn apply_edits(&mut self, mut edits: Vec<TextEdit>, content: &mut Vec<String>) {
        self.push_buffer();
        let actions: Vec<Edit> = edits
            .drain(..)
            .map(|e| Edit::replace_select(e.range.start.into(), e.range.end.into(), e.new_text, content))
            .collect();
        self.push_done(actions);
    }

    fn push_buffer(&mut self) {
        if let Some(action) = self.buffer.collect() {
            self.undone.clear();
            self.push_done(action);
        }
    }

    pub fn indent(&mut self, cursor: &mut Cursor, content: &mut Vec<String>) {
        self.push_buffer();
        let edit = match cursor.select_take() {
            Some((from, to)) => Edit::replace_select(from, to, self.cfg.indent.to_owned(), content),
            None => Edit::insert_clip(cursor.into(), self.cfg.indent.to_owned(), content),
        };
        cursor.add_to_char(self.cfg.indent.len());
        self.push_done(edit);
    }

    pub fn indent_start(&mut self, cursor: &mut Cursor, content: &mut Vec<String>) {
        self.push_buffer();
        let start = CursorPosition { line: cursor.line, char: 0 };
        let edit = Edit::insert_clip(start, self.cfg.indent.to_owned(), content);
        cursor.add_to_char_with_select(self.cfg.indent.len());
        self.push_done(edit);
    }

    pub fn unindent(&mut self, cursor: &mut Cursor, content: &mut [String]) {
        if let Some(line) = content.get_mut(cursor.line) {
            if line.starts_with(&self.cfg.indent) {
                self.push_buffer();
                cursor.checked_sub_char_with_select(self.cfg.indent.len());
                self.push_done(Edit::extract_from_line(cursor.line, 0, self.cfg.indent.len(), line))
            }
        }
    }

    pub fn new_line(&mut self, cursor: &mut Cursor, content: &mut Vec<String>) {
        self.push_buffer();
        let mut builder = EditBuilder::init_alt(cursor, content);
        if content.is_empty() {
            content.push(String::new());
            cursor.line += 1;
            self.push_done(builder.finish(cursor.into(), content));
            return;
        }
        let prev_line = &mut content[cursor.line];
        let mut line = prev_line.split_off(cursor.char);
        let indent = self.cfg.derive_indent_from(prev_line);
        line.insert_str(0, &indent);
        cursor.line += 1;
        cursor.set_char(indent.len());
        // expand scope
        if let Some(last) = prev_line.trim_end().chars().last() {
            if let Some(first) = line.trim_start().chars().next() {
                if [('{', '}'), ('(', ')'), ('[', ']')].contains(&(last, first)) {
                    self.cfg.unindent_if_before_base_pattern(&mut line);
                    let new_char = indent.len() - self.cfg.indent.len();
                    content.insert(cursor.line, line);
                    content.insert(cursor.line, indent);
                    self.push_done(builder.finish((cursor.line + 1, new_char).into(), content));
                    return;
                }
            }
        }
        if prev_line.chars().all(|c| c.is_whitespace()) && prev_line.len().rem_euclid(self.cfg.indent.len()) == 0 {
            builder.and_clear_first_line(prev_line);
        }
        content.insert(cursor.line, line);
        self.push_done(builder.finish(cursor.into(), content));
    }

    pub fn push_char(&mut self, ch: char, cursor: &mut Cursor, content: &mut Vec<String>) {
        if let Some((from, to)) = cursor.select_take() {
            self.push_buffer();
            cursor.set_position(from);
            self.push_done(Edit::remove_select(from, to, content));
        }
        if let Some(line) = content.get_mut(cursor.line) {
            if is_closing_repeat(line.as_str(), ch, cursor.char) {
            } else if let Some(closing) = get_closing_char(ch) {
                let new_text = format!("{ch}{closing}");
                line.insert_str(cursor.char, &new_text);
                self.push_buffer();
                self.push_done(Edit::insertion(cursor.line as u32, cursor.char as u32, new_text));
            } else {
                if let Some(action) = self.buffer.push(cursor.line, cursor.char, ch) {
                    self.push_done(action);
                }
                line.insert(cursor.char, ch);
            }
            cursor.add_to_char(1);
        }
    }

    pub fn del(&mut self, cursor: &mut Cursor, content: &mut Vec<String>) {
        if content.is_empty() {
            return;
        }
        if let Some((from, to)) = cursor.select_take() {
            self.push_buffer();
            cursor.set_position(from);
            self.push_done(Edit::remove_select(from, to, content));
        } else if content[cursor.line].len() == cursor.char {
            self.push_buffer();
            if content.len() > cursor.line + 1 {
                self.push_done(Edit::merge_next_line(cursor.line, content));
            }
        } else if let Some(action) = self.buffer.del(cursor.line, cursor.char, &mut content[cursor.line]) {
            self.push_done(action);
        }
    }

    pub fn backspace(&mut self, cursor: &mut Cursor, content: &mut Vec<String>) {
        if content.is_empty() || cursor.line == 0 && cursor.char == 0 {
            return;
        }
        match cursor.select_take() {
            Some((from, to)) => {
                self.push_buffer();
                cursor.set_position(from);
                self.push_done(Edit::remove_select(from, to, content));
            }
            None if cursor.char == 0 => {
                self.push_buffer();
                cursor.line -= 1;
                let action = Edit::merge_next_line(cursor.line, content);
                cursor.set_char(action.text_edit.range.start.character as usize);
                self.push_done(action);
            }
            None => {
                if let Some(action) =
                    self.buffer.backspace(cursor.line, cursor.char, &mut content[cursor.line], &self.cfg.indent)
                {
                    self.push_done(action);
                }
                cursor.set_char(self.buffer.last_char());
            }
        }
    }

    pub fn undo(&mut self, cursor: &mut Cursor, content: &mut Vec<String>) {
        self.push_buffer();
        if let Some(action) = self.done.pop() {
            let position = action.apply_rev(content, &mut self.events);
            cursor.set_position(position);
            self.undone.push(action);
        }
    }

    pub fn redo(&mut self, cursor: &mut Cursor, content: &mut Vec<String>) {
        self.push_buffer();
        if let Some(action) = self.undone.pop() {
            let position = action.apply(content, &mut self.events);
            cursor.set_position(position);
            self.done.push(action);
        }
    }

    pub fn paste(&mut self, clip: String, cursor: &mut Cursor, content: &mut Vec<String>) {
        self.push_buffer();
        let action = match cursor.select_take() {
            Some((from, to)) => Edit::replace_select(from, to, clip, content),
            None => Edit::insert_clip(cursor.into(), clip, content),
        };
        cursor.set_position(action.end_position());
        self.push_done(action);
    }

    pub fn cut(&mut self, cursor: &mut Cursor, content: &mut Vec<String>) -> String {
        self.push_buffer();
        let action = if let Some((from, to)) = cursor.select_take() {
            cursor.set_position(from);
            Edit::remove_select(from, to, content)
        } else {
            let action = Edit::remove_line(cursor.line, content);
            if cursor.line >= content.len() && content.len() > 1 {
                cursor.line -= 1;
            } else {
                cursor.char = 0;
            }
            action
        };
        let clip = action.reverse_text_edit.new_text.to_owned();
        self.push_done(action);
        clip
    }

    pub fn sync(&mut self, lexer: &mut Lexer, content: &[String]) {
        if let Some(action) = self.buffer.timed_collect() {
            self.push_done(action);
        }
        if !self.events.is_empty() {
            self.version += 1;
            lexer.sync_lsp(self.version, &mut self.events, content);
        }
        self.version += 1;
    }

    pub fn force_sync(&mut self, lexer: &mut Lexer, content: &[String]) {
        self.push_buffer();
        if !self.events.is_empty() {
            self.version += 1;
            lexer.sync_lsp(self.version, &mut self.events, content);
        }
    }

    pub fn push_done(&mut self, edit: impl Into<EditType>) {
        let action: EditType = edit.into();
        action.collect_events(&mut self.events);
        self.done.push(action);
    }
}

#[derive(Debug)]
pub enum EditType {
    Single(Edit),
    Multi(Vec<Edit>),
}

impl EditType {
    pub fn apply_rev(
        &self,
        content: &mut Vec<String>,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
    ) -> CursorPosition {
        match self {
            Self::Single(action) => action.apply_rev(content, events),
            Self::Multi(actions) => {
                actions.iter().rev().map(|a| a.apply_rev(content, events)).last().unwrap_or_default()
            }
        }
    }
    pub fn apply(
        &self,
        content: &mut Vec<String>,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
    ) -> CursorPosition {
        match self {
            Self::Single(action) => action.apply(content, events),
            Self::Multi(actions) => actions.iter().map(|a| a.apply(content, events)).last().unwrap_or_default(),
        }
    }

    pub fn collect_events(&self, events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>) {
        match self {
            Self::Single(action) => events.push((action.meta, action.event())),
            Self::Multi(actions) => {
                for action in actions {
                    events.push((action.meta, action.event()));
                }
            }
        }
    }
}

impl From<Edit> for EditType {
    fn from(value: Edit) -> Self {
        Self::Single(value)
    }
}

impl From<Vec<Edit>> for EditType {
    fn from(value: Vec<Edit>) -> Self {
        Self::Multi(value)
    }
}
