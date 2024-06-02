mod action_buffer;
mod edits;
mod edits_alt;
use crate::{
    configs::IndentConfigs,
    utils::Offset,
    workspace::{
        cursor::{Cursor, CursorPosition, Select},
        line::EditorLine,
        utils::{get_closing_char, is_closing_repeat},
    },
};
use action_buffer::ActionBuffer;
pub use edits::{Edit, EditMetaData, NewLineBuilder};
use lsp_types::{Position, TextDocumentContentChangeEvent, TextEdit};

pub type Events = Vec<(EditMetaData, TextDocumentContentChangeEvent)>;

#[derive(Default)]
pub struct Actions {
    pub cfg: IndentConfigs,
    version: i32,
    done: Vec<EditType>,
    undone: Vec<EditType>,
    events: Events,
    buffer: ActionBuffer,
}

impl Actions {
    pub fn new(cfg: IndentConfigs) -> Self {
        Self { cfg, events: Vec::with_capacity(2), ..Default::default() }
    }

    pub fn swap_up(&mut self, cursor: &mut Cursor, content: &mut [impl EditorLine]) {
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

    pub fn swap_down(&mut self, cursor: &mut Cursor, content: &mut [impl EditorLine]) {
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

    /// Insert new text at the top of the file preserving cursor/select relative position
    pub fn insert_top_cursor_relative_offset(
        &mut self,
        line: String,
        cursor: &mut Cursor,
        content: &mut Vec<impl EditorLine>,
    ) {
        self.push_buffer();
        let edit = Edit::replace_select(CursorPosition::default(), CursorPosition::default(), line, content);
        let offset = edit.meta.to - edit.meta.from;
        cursor.line += offset;
        cursor.at_line += offset;
        cursor.select_line_offset(offset);
        self.push_done(edit);
    }

    pub fn replace_token(&mut self, new: String, cursor: &mut Cursor, content: &mut [impl EditorLine]) {
        self.push_buffer();
        let action = Edit::replace_token(cursor.line, cursor.char, new, content);
        cursor.set_position(action.reverse_text_edit.range.end.into());
        self.push_done(action);
    }

    pub fn replace_select(
        &mut self,
        from: CursorPosition,
        to: CursorPosition,
        clip: impl Into<String>,
        cursor: &mut Cursor,
        content: &mut Vec<impl EditorLine>,
    ) {
        self.push_buffer();
        cursor.select_drop();
        let action = Edit::replace_select(from, to, clip.into(), content);
        cursor.set_position(action.end_position());
        self.push_done(action);
    }

    pub fn insert_snippet(
        &mut self,
        c: &mut Cursor,
        snippet: String,
        cursor_offset: Option<(usize, usize)>,
        content: &mut Vec<impl EditorLine>,
    ) {
        self.push_buffer();
        let (position, action) = Edit::insert_snippet(c, snippet, cursor_offset, &self.cfg, content);
        c.set_position(position);
        self.push_done(action);
    }

    pub fn mass_replace(
        &mut self,
        cursor: &mut Cursor,
        ranges: Vec<Select>,
        clip: String,
        content: &mut Vec<impl EditorLine>,
    ) {
        self.push_buffer();
        cursor.select_drop();
        let actions = ranges
            .into_iter()
            .map(|(from, to)| Edit::replace_select(from, to, clip.to_owned(), content))
            .collect::<Vec<Edit>>();
        if let Some(last) = actions.last() {
            cursor.set_position(last.end_position());
        }
        self.push_done(actions);
    }

    pub fn apply_edits(&mut self, edits: Vec<TextEdit>, content: &mut Vec<impl EditorLine>) {
        self.push_buffer();
        let actions = edits
            .into_iter()
            .map(|e| Edit::replace_select(e.range.start.into(), e.range.end.into(), e.new_text, content))
            .collect::<Vec<Edit>>();
        self.push_done(actions);
    }

    pub fn indent(&mut self, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) {
        self.push_buffer();
        match cursor.select_take() {
            Some((from, to)) => {
                if from.line == to.line {
                    self.push_done(Edit::replace_select(from, to, self.cfg.indent.to_owned(), content));
                } else {
                    let edits = self.indent_range(cursor, from, to, content);
                    self.push_done(edits);
                }
            }
            None => {
                self.push_done(Edit::insert_clip(cursor.into(), self.cfg.indent.to_owned(), content));
                cursor.add_to_char(self.cfg.indent.len());
            }
        }
    }

    pub fn indent_start(&mut self, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) {
        self.push_buffer();
        match cursor.select_take() {
            Some((from, to)) => {
                let edits = self.indent_range(cursor, from, to, content);
                self.push_done(edits);
            }
            None => {
                let start = CursorPosition { line: cursor.line, char: 0 };
                let edit = Edit::insert_clip(start, self.cfg.indent.to_owned(), content);
                cursor.add_to_char(self.cfg.indent.len());
                self.push_done(edit);
            }
        }
    }

    fn indent_range(
        &mut self,
        cursor: &mut Cursor,
        mut from: CursorPosition,
        mut to: CursorPosition,
        content: &mut [impl EditorLine],
    ) -> Vec<Edit> {
        let initial_select = (from, to);
        if from.char != 0 {
            from.char += self.cfg.indent.len();
        }
        let mut edit_lines = to.line - from.line;
        if to.char != 0 {
            to.char += self.cfg.indent.len();
            edit_lines += 1;
        };
        cursor.select_set(from, to);
        let mut edits = Vec::with_capacity(edit_lines);
        for (line_idx, text) in content.iter_mut().enumerate().skip(from.line).take(edit_lines) {
            text.insert_str(0, &self.cfg.indent);
            let position = Position::new(line_idx as u32, 0);
            edits.push(Edit::record_in_line_insertion(position, self.cfg.indent.to_owned()))
        }
        add_select(&mut edits, Some(initial_select), Some((from, to)));
        edits
    }

    pub fn unindent(&mut self, cursor: &mut Cursor, content: &mut [impl EditorLine]) {
        self.push_buffer();
        match cursor.select_take() {
            Some((mut from, mut to)) => {
                let initial_select = (from, to);
                let mut edit_lines = to.line - from.line;
                if to.char != 0 {
                    // include last line only if part of it is selected
                    edit_lines += 1;
                }
                let mut edits = Vec::new();
                for (line_idx, text) in content.iter_mut().enumerate().skip(from.line).take(edit_lines) {
                    if let Some((offset, edit)) = Edit::unindent(line_idx, text, &self.cfg.indent) {
                        if from.line == line_idx {
                            from.char = offset.offset(from.char);
                        }
                        if to.line == line_idx {
                            to.char = offset.offset(to.char);
                        }
                        edits.push(edit);
                    };
                }
                cursor.select_set(from, to);
                add_select(&mut edits, Some(initial_select), Some((from, to)));
                self.push_done(edits);
            }
            None => {
                let _ = content
                    .get_mut(cursor.line)
                    .and_then(|text| Edit::unindent(cursor.line, text, &self.cfg.indent))
                    .map(|(offset, edit)| {
                        self.push_done(edit);
                        cursor.char = offset.offset(cursor.char);
                    });
            }
        }
    }

    pub fn new_line(&mut self, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) {
        self.push_buffer();
        let mut builder = NewLineBuilder::new(cursor, content);
        if content.is_empty() {
            content.push(String::new().into());
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
        if let Some(opening) = prev_line.trim_end().chars().last() {
            if let Some(closing) = line.trim_start().chars().next() {
                if [('{', '}'), ('(', ')'), ('[', ']')].contains(&(opening, closing)) {
                    self.cfg.unindent_if_before_base_pattern(&mut line);
                    let new_char = indent.len() - self.cfg.indent.len();
                    content.insert(cursor.line, line);
                    content.insert(cursor.line, indent.into());
                    self.push_done(builder.finish((cursor.line + 1, new_char).into(), content));
                    return;
                }
            }
        }
        if prev_line.chars().all(|c| c.is_whitespace()) && prev_line.char_len().rem_euclid(self.cfg.indent.len()) == 0 {
            builder.and_clear_first_line(prev_line);
        }
        content.insert(cursor.line, line);
        self.push_done(builder.finish(cursor.into(), content));
    }

    pub fn comment_out(&mut self, pat: &str, cursor: &mut Cursor, content: &mut [impl EditorLine]) {
        // TODO refactor
        match cursor.select_take() {
            Some((mut from, mut to)) => {
                let from_char = from.char;
                let lines_n = to.line - from.line + 1;
                let cb = if select_is_commented(from.line, lines_n, pat, content) { uncomment } else { into_comment };
                let select = content.iter_mut().enumerate().skip(from.line).take(lines_n);
                let edits = select
                    .flat_map(|(line_idx, line)| {
                        (cb)(pat, line, CursorPosition { line: line_idx, char: cursor.char }).map(|(offset, edit)| {
                            if to.line == line_idx {
                                to.char = offset.offset(to.char);
                            }
                            if from.line == line_idx {
                                from.char = offset.offset(from.char);
                            }
                            edit
                        })
                    })
                    .collect::<Vec<Edit>>();
                if from.line == to.line {
                    if from_char == cursor.char {
                        cursor.select_set(to, from);
                    } else {
                        cursor.select_set(from, to);
                    }
                } else if from.line == cursor.line {
                    cursor.select_set(to, from);
                } else {
                    cursor.select_set(from, to);
                };
                self.push_done(edits);
            }
            _ => {
                let line = &mut content[cursor.line];
                if let Some((offset, edit)) = uncomment(pat, line, cursor.into()) {
                    self.push_done(edit);
                    cursor.char = offset.offset(cursor.char);
                } else if let Some((offset, edit)) = into_comment(pat, line, cursor.into()) {
                    self.push_done(edit);
                    cursor.char = offset.offset(cursor.char);
                }
            }
        }
    }

    pub fn push_char(&mut self, ch: char, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) {
        match cursor.select_take() {
            Some((mut from, mut to)) => {
                self.push_buffer();
                match get_closing_char(ch) {
                    Some(closing) => {
                        content[to.line].insert(to.char, closing);
                        content[from.line].insert(from.char, ch);
                        let first_edit = Edit::record_in_line_insertion(to.into(), closing.into()).select(from, to);
                        let second_edit = Edit::record_in_line_insertion(from.into(), ch.to_string());
                        from.char += 1;
                        if from.line == to.line {
                            to.char += 1;
                        }
                        self.push_done(vec![first_edit, second_edit.new_select(from, to)]);
                        cursor.set_position(to);
                        cursor.select_set(from, to);
                    }
                    None => {
                        cursor.set_position(from);
                        self.push_done(Edit::remove_select(from, to, content));
                        self.push_char_simple(ch, cursor, content);
                    }
                }
            }
            None => self.push_char_simple(ch, cursor, content),
        }
    }

    fn push_char_simple(&mut self, ch: char, cursor: &mut Cursor, content: &mut [impl EditorLine]) {
        if let Some(line) = content.get_mut(cursor.line) {
            if is_closing_repeat(line, ch, cursor.char) {
            } else if let Some(closing) = get_closing_char(ch) {
                let new_text = format!("{ch}{closing}");
                line.insert_str(cursor.char, &new_text);
                self.push_buffer();
                self.push_done(Edit::record_in_line_insertion(cursor.into(), new_text));
            } else {
                if let Some(edit) = self.buffer.push(cursor.line, cursor.char, ch) {
                    self.push_done(edit);
                }
                line.insert(cursor.char, ch);
            }
            cursor.add_to_char(1);
        }
    }

    pub fn del(&mut self, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) {
        if content.is_empty() {
            return;
        }
        match cursor.select_take() {
            Some((from, to)) => {
                self.push_buffer();
                cursor.set_position(from);
                self.push_done(Edit::remove_select(from, to, content));
            }
            None if content[cursor.line].char_len() == cursor.char => {
                self.push_buffer();
                if content.len() > cursor.line + 1 {
                    self.push_done(Edit::merge_next_line(cursor.line, content));
                }
            }
            None => {
                let _ = self
                    .buffer
                    .del(cursor.line, cursor.char, &mut content[cursor.line])
                    .map(|edit| self.push_done(edit));
            }
        }
    }

    pub fn backspace(&mut self, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) {
        if content.is_empty() || cursor.line == 0 && cursor.char == 0 && cursor.select_is_none() {
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
                let edit = Edit::merge_next_line(cursor.line, content);
                cursor.set_char(edit.text_edit.range.start.character as usize);
                self.push_done(edit);
            }
            None => {
                let _ = self
                    .buffer
                    .backspace(cursor.line, cursor.char, &mut content[cursor.line], &self.cfg.indent)
                    .map(|edit| self.push_done(edit));
                cursor.set_char(self.buffer.last_char());
            }
        }
    }

    pub fn undo(&mut self, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) {
        self.push_buffer();
        if let Some(action) = self.done.pop() {
            let (position, select) = action.apply_rev(content, &mut self.events);
            cursor.set_position(position);
            cursor.select_replace(select);
            self.undone.push(action);
        }
    }

    pub fn redo(&mut self, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) {
        self.push_buffer();
        if let Some(action) = self.undone.pop() {
            let (position, select) = action.apply(content, &mut self.events);
            cursor.set_position(position);
            cursor.select_replace(select);
            self.done.push(action);
        }
    }

    pub fn paste(&mut self, clip: String, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) {
        self.push_buffer();
        let edit = match cursor.select_take() {
            Some((from, to)) => Edit::replace_select(from, to, clip, content),
            None => Edit::insert_clip(cursor.into(), clip, content),
        };
        cursor.set_position(edit.end_position());
        self.push_done(edit);
    }

    pub fn cut(&mut self, cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) -> String {
        self.push_buffer();
        let edit = if let Some((from, to)) = cursor.select_take() {
            cursor.set_position(from);
            Edit::remove_select(from, to, content)
        } else {
            let action = Edit::remove_line(cursor.line, content);
            if content.is_empty() {
                content.push(Default::default());
                cursor.line = 0;
            } else if cursor.line >= content.len() && content.len() > 1 {
                cursor.line -= 1;
            }
            cursor.char = 0;
            action
        };
        let clip = edit.get_removed_text().to_owned();
        self.push_done(edit);
        clip
    }

    pub fn get_events(&mut self) -> Option<(i32, &mut Events)> {
        if let Some(action) = self.buffer.timed_collect() {
            self.push_done(action);
        }
        if self.events.is_empty() {
            return None;
        }
        self.version += 1;
        Some((self.version, &mut self.events))
    }

    pub fn get_events_versionless(&mut self) -> Option<&mut Events> {
        if let Some(action) = self.buffer.timed_collect() {
            self.push_done(action);
        }
        if self.events.is_empty() {
            return None;
        }
        Some(&mut self.events)
    }

    pub fn get_events_forced(&mut self) -> (i32, &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>) {
        self.push_buffer();
        if !self.events.is_empty() {
            self.version += 1;
        }
        (self.version, &mut self.events)
    }

    fn push_done(&mut self, edit: impl Into<EditType>) {
        let action: EditType = edit.into();
        action.collect_events(&mut self.events);
        self.done.push(action);
    }

    pub fn push_buffer(&mut self) {
        if let Some(action) = self.buffer.collect() {
            self.undone.clear();
            self.push_done(action);
        }
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
        content: &mut Vec<impl EditorLine>,
        events: &mut Events,
    ) -> (CursorPosition, Option<Select>) {
        match self {
            Self::Single(action) => action.apply_rev(content, events),
            Self::Multi(actions) => {
                actions.iter().rev().map(|a| a.apply_rev(content, events)).last().unwrap_or_default()
            }
        }
    }

    pub fn apply(&self, content: &mut Vec<impl EditorLine>, events: &mut Events) -> (CursorPosition, Option<Select>) {
        match self {
            Self::Single(action) => action.apply(content, events),
            Self::Multi(actions) => actions.iter().map(|a| a.apply(content, events)).last().unwrap_or_default(),
        }
    }

    pub fn collect_events(&self, events: &mut Events) {
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

#[inline]
fn add_select(edits: &mut [Edit], old: Option<Select>, new: Option<Select>) {
    if let Some(edit) = edits.first_mut() {
        edit.select = old;
    }
    if let Some(edit) = edits.last_mut() {
        edit.select = new;
    }
}

#[inline]
fn select_is_commented(from: usize, n: usize, pat: &str, content: &[impl EditorLine]) -> bool {
    content.iter().skip(from).take(n).all(|l| l.trim_start().starts_with(pat) || l.chars().all(|c| c.is_whitespace()))
}

#[inline]
fn into_comment(pat: &str, line: &mut impl EditorLine, cursor: CursorPosition) -> Option<(Offset, Edit)> {
    let idx = line.char_indices().flat_map(|(idx, c)| if c.is_whitespace() { None } else { Some(idx) }).next()?;
    let comment_start = format!("{pat} ");
    line.insert_str(idx, &comment_start);
    let offset = if cursor.char >= idx { Offset::Pos(comment_start.len()) } else { Offset::Pos(0) };
    Some((
        offset,
        Edit::record_in_line_insertion(CursorPosition { line: cursor.line, char: idx }.into(), comment_start),
    ))
}

#[inline]
fn uncomment(pat: &str, line: &mut impl EditorLine, cursor: CursorPosition) -> Option<(Offset, Edit)> {
    if !line.trim_start().starts_with(pat) {
        return None;
    }
    let idx = line.find(pat)?;
    let mut end_idx = idx + pat.len();
    end_idx += line[idx + pat.len()..].chars().take_while(|c| c.is_whitespace()).count();
    let offset = if cursor.char >= idx { Offset::Neg(end_idx - idx) } else { Offset::Neg(0) };
    Some((offset, Edit::remove_from_line(cursor.line, idx, end_idx, line)))
}

#[cfg(test)]
pub mod tests;
