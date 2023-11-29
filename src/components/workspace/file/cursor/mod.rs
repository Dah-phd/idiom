mod action_buffer;
mod actions;
mod position;
mod select;
use action_buffer::ActionBuffer;
use actions::{Action, ActionBuilder};
use lsp_types::TextDocumentContentChangeEvent;

use crate::configs::EditorConfigs;
pub use position::CursorPosition;
pub use select::Select;

use super::utils::{get_closing_char, insert_clip, is_closing_repeat, remove_content};

#[derive(Debug)]
pub struct Cursor {
    pub line: usize,
    pub char: usize,
    pub select: Select,
    pub cfg: EditorConfigs,
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
            cfg: configs.clone(),
            done: Vec::default(),
            undone: Vec::default(),
            version: 0,
            events: Vec::default(),
            buffer: ActionBuffer::default(),
        }
    }

    pub fn source_configs(&mut self, cfg: &EditorConfigs) {
        self.cfg = cfg.clone();
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

    pub fn swap_down(&mut self, up_idx: usize, content: &mut [String]) {
        self.drop_select();
        self.push_buffer();
        self.push_done(Action::swap_down(up_idx, &self.cfg, content));
    }

    pub fn replace_token(&mut self, new: String, content: &mut [String]) {
        let action = Action::replace_token(self.line, self.char, new, content);
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
            builder.raw_finish(
                CursorPosition { line: self.line, char: self.cfg.indent.len() },
                self.cfg.indent.to_owned(),
            ),
        )
    }

    pub fn unindent(&mut self, content: &mut [String]) {
        if let Some(line) = content.get_mut(self.line) {
            if line.starts_with(&self.cfg.indent) {
                self.push_buffer();
                let mut old_text = line.split_off(self.cfg.indent.len());
                std::mem::swap(line, &mut old_text);
                self.char = self.char.checked_sub(self.cfg.indent.len()).unwrap_or_default();
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
        let indent = self.cfg.derive_indent_from(prev_line);
        line.insert_str(0, &indent);
        self.line += 1;
        self.char = indent.len();
        // expand scope
        if let Some(last) = prev_line.trim_end().chars().last() {
            if let Some(first) = line.trim_start().chars().next() {
                if [('{', '}'), ('(', ')'), ('[', ']')].contains(&(last, first)) {
                    self.cfg.unindent_if_before_base_pattern(&mut line);
                    let new_char = indent.len() - self.cfg.indent.len();
                    content.insert(self.line, line);
                    content.insert(self.line, indent);
                    self.push_done(builder.finish((self.line + 1, new_char).into(), content));
                    return;
                }
            }
        }
        if prev_line.chars().all(|c| c.is_whitespace()) && prev_line.len().rem_euclid(self.cfg.indent.len()) == 0 {
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
                if let Some(action) = self.buffer.push(self.line, self.char, ch) {
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
            self.push_buffer();
            self.set_position(from);
            self.push_done(ActionBuilder::cut_range(from, to, content).force_finish());
        } else if content[self.line].len() == self.char {
            self.push_buffer();
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
            self.push_buffer();
            self.set_position(from);
            self.push_done(ActionBuilder::cut_range(from, to, content).force_finish());
        } else if self.char == 0 {
            self.push_buffer();
            self.line -= 1;
            let action = Action::merge_next_line(self.line, content);
            self.char = action.text_edit.range.start.character as usize;
            self.push_done(action);
        } else {
            if let Some(action) = self.buffer.backspace(self.line, self.char, &mut content[self.line], &self.cfg.indent)
            {
                self.push_done(action);
            }
            self.char = self.buffer.last_char();
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
            let action = ActionBuilder::cut_line(self.line, content);
            if self.line >= content.len() && content.len() != 1 {
                self.line -= 1;
                self.char = content[self.line].len();
            } else {
                self.char = 0;
            }
            action
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
            self.events.push(action.reverse_event());
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
            line.insert_str(idx, &self.cfg.indent);
            self.char += self.cfg.indent.len();
        } else {
            content.insert(self.line, self.cfg.indent.to_owned());
            self.char = self.cfg.indent.len();
        }
    }

    fn push_buffer(&mut self) {
        if let Some(action) = self.buffer.collect() {
            self.undone.clear();
            self.push_done(action);
        }
    }
}
