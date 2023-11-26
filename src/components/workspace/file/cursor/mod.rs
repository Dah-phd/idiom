mod actions;
mod position;
mod select;
use actions::{Action, ActionBuffer, ActionBuilder};
use lsp_types::TextDocumentContentChangeEvent;

use crate::{configs::EditorConfigs, utils::trim_start_inplace};
pub use position::{CursorPosition, Offset};
pub use select::Select;

use super::utils::{get_closing_char, insert_clip, is_closing_repeat, remove_content};

#[derive(Debug)]
pub struct Cursor {
    pub line: usize,
    pub char: usize,
    pub select: Select,
    indent: String,
    indent_after: String,
    unindent_before: String,
    done: Vec<Action>,
    undone: Vec<Action>,
    version: i32,
    events: Vec<TextDocumentContentChangeEvent>,
    buffer: ActionBuffer,
}

impl Cursor {
    pub fn new(configs: EditorConfigs) -> Self {
        Self {
            line: 0,
            char: 0,
            select: Select::None,
            indent: configs.indent,
            indent_after: configs.indent_after,
            unindent_before: configs.unindent_before,
            done: Vec::default(),
            undone: Vec::default(),
            version: 0,
            events: Vec::default(),
            buffer: ActionBuffer::default(),
        }
    }

    pub fn source_configs(&mut self, cfg: &EditorConfigs) {
        self.indent = cfg.indent.to_owned();
        self.indent_after = cfg.indent_after.to_owned();
        self.unindent_before = cfg.unindent_before.to_owned();
    }

    pub fn init_select(&mut self) {
        self.select.init(self.line, self.char);
    }

    pub fn push_to_select(&mut self) {
        self.select.push(&self.position());
    }

    pub fn drop_select(&mut self) {
        self.select = Select::None;
    }

    pub fn position(&self) -> CursorPosition {
        CursorPosition { line: self.line, char: self.char }
    }

    pub fn set_position(&mut self, positon: CursorPosition) {
        self.line = positon.line;
        self.char = positon.char;
    }

    pub fn get_text_edits(&mut self) -> Option<(i32, Vec<TextDocumentContentChangeEvent>)> {
        if let Some(action) = self.buffer.timed_collect() {
            self.push_done(action);
        }
        if self.events.is_empty() {
            return None;
        }
        self.version += 1;
        Some((self.version, std::mem::take(&mut self.events)))
    }

    // UTILS

    pub fn swap_down(&mut self, up_idx: usize, content: &mut [String]) -> (Offset, Offset) {
        self.push_buffer();
        let to = up_idx + 1;
        let builder = ActionBuilder::for_swap(content, up_idx);
        content.swap(up_idx, to);
        let offset = self.indent_line(up_idx, content);
        let offset2 = self.indent_line(to, content);
        self.push_done(builder.finish_swap(content));
        (offset, offset2)
    }

    pub fn replace_token(&mut self, new: String, content: &mut [String]) {
        let action = Action::replace_range(self.line, self.char, new, content);
        self.char = action.reverse_text_edit.range.end.character as usize;
        self.push_buffer();
        self.push_done(action);
    }

    pub fn replace_select(&mut self, select: Select, new_clip: impl Into<String>, content: &mut Vec<String>) {
        if let Some((from, to)) = select.try_unwrap() {
            self.push_buffer();
            let clip = new_clip.into();
            self.drop_select();
            let builder = ActionBuilder::cut_range(from, to, content);
            let end_point = insert_clip(clip.clone(), content, from);
            self.push_done(builder.push_clip(clip, &end_point));
            self.set_position(end_point);
        }
    }

    pub fn push_done(&mut self, action: Action) {
        self.events.push(action.event());
        self.done.push(action);
    }

    pub fn indent(&mut self, content: &mut Vec<String>) {
        let builder = ActionBuilder::init(self, content);
        self.indent_at(self.char, content);
        self.push_done(builder.finish(self.position(), content));
    }

    pub fn indent_start(&mut self, content: &mut Vec<String>) {
        let builder = ActionBuilder::empty_at(CursorPosition { line: self.line, char: 0 });
        self.indent_at(0, content);
        self.push_done(
            builder.raw_finish(CursorPosition { line: self.line, char: self.indent.len() }, self.indent.to_owned()),
        )
    }

    pub fn unindent(&mut self, content: &mut [String]) {
        if let Some(line) = content.get_mut(self.line) {
            if line.starts_with(&self.indent) {
                self.push_buffer();
                let mut old_text = line.split_off(self.indent.len());
                std::mem::swap(line, &mut old_text);
                self.char = self.char.checked_sub(self.indent.len()).unwrap_or_default();
                self.push_done(Action::extract(self.line as u32, 0, old_text))
            }
        }
    }

    pub fn new_line(&mut self, content: &mut Vec<String>) {
        self.push_buffer();
        let mut builder = ActionBuilder::init(self, content);
        if content.is_empty() {
            content.push(String::new());
            self.line += 1;
            self.push_done(builder.finish(self.position(), content));
            return;
        }
        let prev_line = &mut content[self.line];
        let mut line = prev_line.split_off(self.char);
        let indent = self.derive_indent_from(prev_line);
        line.insert_str(0, &indent);
        self.line += 1;
        self.char = indent.len();
        // expand scope
        if let Some(last) = prev_line.trim_end().chars().last() {
            if let Some(first) = line.trim_start().chars().next() {
                if [('{', '}'), ('(', ')'), ('[', ']')].contains(&(last, first)) {
                    self.unindent_if_before_base_pattern(&mut line);
                    let new_char = indent.len() - self.indent.len();
                    content.insert(self.line, line);
                    content.insert(self.line, indent);
                    self.push_done(builder.finish((self.line + 1, new_char).into(), content));
                    return;
                }
            }
        }
        if prev_line.chars().all(|c| c.is_whitespace()) && prev_line.len().rem_euclid(self.indent.len()) == 0 {
            builder.and_clear_first_line(prev_line);
        }
        content.insert(self.line, line);
        self.push_done(builder.finish(self.position(), content));
    }

    pub fn push_char(&mut self, ch: char, content: &mut Vec<String>) {
        if let Some((from, to)) = self.select.take_option() {
            self.push_buffer();
            let builder = ActionBuilder::cut_range(from, to, content);
            self.push_done(builder.force_finish());
            self.set_position(from);
        }
        if let Some(line) = content.get_mut(self.line) {
            if is_closing_repeat(line.as_str(), ch, self.char) {
                self.char += 1;
                return;
            }
            if let Some(closing) = get_closing_char(ch) {
                let new_text = format!("{ch}{closing}");
                line.insert_str(self.char, &new_text);
                self.push_buffer();
                self.push_done(Action::insertion(self.line as u32, self.char as u32, new_text));
            } else {
                if let Some(action) = self.buffer.push(ch, self.line, self.char) {
                    self.push_done(action);
                }
                line.insert(self.char, ch);
            }
            self.char += 1;
        } else {
            content.insert(self.line, ch.to_string());
            self.char = 1;
        }
    }

    pub fn del(&mut self, content: &mut Vec<String>) {
        if content.is_empty() {
            return;
        }
        if let Some((from, to)) = self.select.take_option() {
            self.set_position(from);
            self.push_done(ActionBuilder::cut_range(from, to, content).force_finish());
        } else if content[self.line].len() == self.char {
            if content.len() > self.line + 1 {
                self.push_done(Action::merge_next_line(self.line, content));
            }
        } else if let Some(action) = self.buffer.del(self.line, self.char, &mut content[self.line]) {
            self.push_done(action);
        }
    }

    pub fn backspace(&mut self, content: &mut Vec<String>) {
        if content.is_empty() || self.line == 0 && self.char == 0 {
            return;
        }
        if let Some((from, to)) = self.select.take_option() {
            self.set_position(from);
            self.push_done(ActionBuilder::cut_range(from, to, content).force_finish());
        } else if self.char == 0 {
            self.line -= 1;
            let action = Action::merge_next_line(self.line, content);
            self.char = action.text_edit.range.start.character as usize;
            self.push_done(action);
        } else {
            if let Some(action) = self.buffer.backspace(self.line, self.char, &mut content[self.line], &self.indent) {
                self.push_done(action);
            }
            self.char = self.buffer.last_char;
        }
    }

    pub fn paste(&mut self, clip: String, content: &mut Vec<String>) {
        self.push_buffer();
        let builder = if let Some((from, to)) = self.select.take_option() {
            self.set_position(from);
            ActionBuilder::cut_range(from, to, content)
        } else {
            ActionBuilder::empty_at(self.position())
        };
        let end_position = insert_clip(clip.clone(), content, self.position());
        self.push_done(builder.push_clip(clip, &end_position));
        self.set_position(end_position);
    }

    pub fn cut(&mut self, content: &mut Vec<String>) -> String {
        self.push_buffer();
        let builder = if let Some((from, to)) = self.select.take_option() {
            self.set_position(from);
            ActionBuilder::cut_range(from, to, content)
        } else {
            ActionBuilder::cut_line(self.line, content)
        };
        let clip = builder.reverse_edit_text.to_owned();
        self.push_done(builder.force_finish());
        clip
    }

    pub fn undo(&mut self, content: &mut Vec<String>) {
        self.push_buffer();
        if let Some(action) = self.done.pop() {
            let from = action.reverse_text_edit.range.start.into();
            let to = action.reverse_text_edit.range.end.into();
            self.set_position(from);
            remove_content(&from, &to, content);
            insert_clip(action.reverse_text_edit.new_text.to_owned(), content, from);
            self.events.push(action.reverse());
            self.undone.push(action);
        }
    }

    pub fn redo(&mut self, content: &mut Vec<String>) {
        self.push_buffer();
        if let Some(action) = self.undone.pop() {
            let from = action.text_edit.range.start.into();
            let to = action.text_edit.range.end.into();
            self.set_position(from);
            remove_content(&from, &to, content);
            insert_clip(action.text_edit.new_text.to_owned(), content, from);
            self.events.push(action.event());
            self.done.push(action);
        }
    }

    fn indent_at(&mut self, idx: usize, content: &mut Vec<String>) {
        self.push_buffer();
        if let Some(line) = content.get_mut(self.line) {
            line.insert_str(idx, &self.indent);
            self.char += self.indent.len();
        } else {
            content.insert(self.line, self.indent.to_owned());
            self.char = self.indent.len();
        }
    }

    fn push_buffer(&mut self) {
        if let Some(action) = self.buffer.collect() {
            self.undone.clear();
            self.push_done(action);
        }
    }

    pub fn offset_char(&mut self, offset: Offset) {
        match offset {
            Offset::Neg(val) => self.char = self.char.checked_sub(val).unwrap_or_default(),
            Offset::Pos(val) => self.char += val,
        }
    }

    fn unindent_if_before_base_pattern(&self, line: &mut String) -> Offset {
        if line.starts_with(&self.indent) {
            if let Some(first) = line.trim_start().chars().next() {
                if self.unindent_before.contains(first) {
                    line.replace_range(..self.indent.len(), "");
                    return Offset::Neg(self.indent.len());
                }
            }
        }
        Offset::Pos(0)
    }

    fn derive_indent_from(&self, prev_line: &str) -> String {
        let mut indent = prev_line.chars().take_while(|&c| c.is_whitespace()).collect::<String>();
        if let Some(last) = prev_line.trim_end().chars().last() {
            if self.indent_after.contains(last) {
                indent.insert_str(0, &self.indent);
            }
        };
        indent
    }

    pub fn indent_from_prev(&self, prev_line: &str, line: &mut String) -> Offset {
        let indent = self.derive_indent_from(prev_line);
        let offset = trim_start_inplace(line) + indent.len();
        line.insert_str(0, &indent);
        offset + self.unindent_if_before_base_pattern(line)
    }

    fn indent_line(&mut self, line_idx: usize, content: &mut [String]) -> Offset {
        if line_idx > 0 {
            let (prev_split, current_split) = content.split_at_mut(line_idx);
            let prev = &prev_split[line_idx - 1];
            let line = &mut current_split[0];
            self.indent_from_prev(prev, line)
        } else {
            let line = &mut content[line_idx];
            trim_start_inplace(line)
        }
    }
}
