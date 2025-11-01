mod event;
mod methods;

use crate::{
    configs::EditorAction,
    global_state::GlobalState,
    syntax::Lexer,
    workspace::{
        actions::{transaction, Actions},
        cursor::{Cursor, CursorPosition, PositionedWord},
        Editor, EditorLine,
    },
};
use lsp_types::TextEdit;

/// holds controls and manages cursor type
/// if switched to multicursor will swap callbacks to multi cursor
pub struct ControlMap {
    pub action_map: fn(&mut Editor, EditorAction, gs: &mut GlobalState) -> bool,
    pub insert_import: fn(&mut Editor, String),
    pub insert_snippet: fn(&mut Editor, String, Option<(usize, usize)>),
    pub insert_snippet_with_select: fn(&mut Editor, String, (usize, usize), usize),
    pub replace_token: fn(&mut Editor, String),
    pub replace_select: fn(&mut Editor, CursorPosition, CursorPosition, &str),
    pub mass_replace: fn(&mut Editor, Vec<(CursorPosition, CursorPosition)>, String),
    pub apply_file_edits: fn(&mut Editor, Vec<TextEdit>),
    pub copy: fn(&mut Editor) -> Option<String>,
    pub cut: fn(&mut Editor) -> Option<String>,
    pub paste: fn(&mut Editor, String),
    cursors: Vec<Cursor>,
}

impl ControlMap {
    #[inline]
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    #[inline]
    pub fn cursors_count(&self) -> usize {
        self.cursors.len()
    }

    #[inline]
    pub fn is_multicursor(&self) -> bool {
        !self.cursors.is_empty()
    }

    #[inline]
    pub fn set_cursors_text_width(&mut self, text_width: usize) {
        for cursor in self.cursors.iter_mut() {
            cursor.text_width = text_width;
        }
    }

    pub fn get_base_cursor_position(&self) -> Option<CursorPosition> {
        for cursor in self.cursors.iter() {
            if cursor.max_rows != 0 {
                return Some(cursor.get_position());
            }
        }
        None
    }

    pub fn apply<F>(editor: &mut Editor, mut callback: F)
    where
        F: FnMut(&mut Actions, &mut Lexer, &mut Vec<EditorLine>, &mut Cursor),
    {
        let controls = &mut editor.controls;
        if controls.cursors.is_empty() {
            (callback)(&mut editor.actions, &mut editor.lexer, &mut editor.content, &mut editor.cursor);
        } else {
            apply_multi_cursor_transaction(editor, callback);
            Self::consolidate_cursors(editor);
        }
    }

    pub fn try_multi_cursor(editor: &mut Editor) -> bool {
        if !editor.renderer.try_multi_cursor(editor.file_type) {
            return false;
        };
        editor.controls.cursors.clear();
        editor.controls.cursors.push(editor.cursor.clone());
        editor.controls.multi_cursor_map();
        true
    }

    pub fn single_cursor(editor: &mut Editor) {
        match editor.controls.cursors.iter().find(|c| c.max_rows != 0) {
            Some(cursor) => editor.cursor.set_cursor(cursor),
            None => editor.cursor.set_position(CursorPosition::default()),
        };
        editor.last_render_at_line = None;
        editor.controls.single_cursor_map();
        editor.renderer.single_cursor(editor.file_type);
    }

    pub fn ensure_single_cursor(editor: &mut Editor) {
        if editor.controls.cursors.is_empty() {
            return;
        }
        Self::single_cursor(editor);
    }

    pub fn force_singel_cursor_reset(editor: &mut Editor) {
        editor.cursor.reset();
        if editor.controls.cursors.is_empty() {
            return;
        }
        editor.controls.cursors.clear();
        editor.controls.single_cursor_map();
        editor.renderer.single_cursor(editor.file_type);
    }

    pub fn consolidate_cursors(editor: &mut Editor) {
        let mut idx = 1;

        let cursors = &mut editor.controls.cursors;
        cursors.sort_by(sort_cursors);

        while idx < cursors.len() {
            unsafe {
                let [cursor, other] = cursors.get_disjoint_unchecked_mut([idx - 1, idx]);
                if cursor.merge_if_intersect(other) {
                    cursor.max_rows = std::cmp::max(cursor.max_rows, other.max_rows);
                    cursors.remove(idx);
                } else {
                    idx += 1;
                }
            }
        }
        if cursors.len() < 2 {
            ControlMap::single_cursor(editor);
        }
    }

    pub fn consolidate_cursors_per_line(editor: &mut Editor) {
        let mut idx = 1;
        let cursors = &mut editor.controls.cursors;
        cursors.sort_by(sort_cursors);

        while idx < cursors.len() {
            unsafe {
                let [cursor, other] = cursors.get_disjoint_unchecked_mut([idx - 1, idx]);
                if cursor.line == other.line {
                    cursor.max_rows = std::cmp::max(cursor.max_rows, other.max_rows);
                    cursors.remove(idx);
                } else {
                    idx += 1;
                }
            }
        }
        if cursors.len() < 2 {
            ControlMap::single_cursor(editor);
        }
    }

    fn multi_cursor_map(&mut self) {
        self.action_map = event::multi_cursor_map;

        self.insert_import = methods::multic_insert_import;
        self.replace_token = methods::multic_replace_token;
        self.insert_snippet = methods::multic_insert_snippet;
        self.insert_snippet_with_select = methods::multic_insert_snippet_with_select;

        self.cut = methods::multic_cut;
        self.copy = methods::multic_copy;
        self.paste = methods::multic_paste;
    }

    fn single_cursor_map(&mut self) {
        self.cursors.clear();
        self.action_map = event::single_cursor_map;

        self.insert_import = methods::insert_import;
        self.replace_token = methods::replace_token;
        self.insert_snippet = methods::insert_snippet;
        self.insert_snippet_with_select = methods::insert_snippet_with_select;

        self.cut = methods::cut;
        self.copy = methods::copy;
        self.paste = methods::paste;
    }
}

impl Default for ControlMap {
    fn default() -> Self {
        Self {
            action_map: event::single_cursor_map,
            cursors: Vec::default(),
            insert_snippet_with_select: methods::insert_snippet_with_select,
            insert_snippet: methods::insert_snippet,
            insert_import: methods::insert_import,
            replace_token: methods::replace_token,
            replace_select: methods::replace_select,
            mass_replace: methods::mass_replace,
            apply_file_edits: methods::apply_file_edits,
            cut: methods::cut,
            copy: methods::copy,
            paste: methods::paste,
        }
    }
}

pub fn filter_multi_cursors_per_line_if_no_select(editor: &Editor) -> Vec<Cursor> {
    let mut filtered = vec![];
    let mut index = 0;
    let cursors = &editor.controls.cursors;
    loop {
        let Some(mut cursor) = cursors.get(index).cloned() else {
            return filtered;
        };
        if cursor.select_is_none() {
            // remove any cursors already added on the same line
            while let Some(last_filtered) = filtered.last() {
                if last_filtered.line != cursor.line {
                    break;
                }
                cursor.max_rows = std::cmp::max(cursor.max_rows, last_filtered.max_rows);
                filtered.pop();
            }
            // skip all cursors following on the same line
            index += 1;
            while let Some(next_cursor) = cursors.get(index) {
                if next_cursor.line != cursor.line {
                    break;
                }
                cursor.max_rows = std::cmp::max(cursor.max_rows, next_cursor.max_rows);
                index += 1;
            }
        } else {
            index += 1;
        };
        filtered.push(cursor);
    }
}

pub fn push_multicursor_position(editor: &mut Editor, mut position: CursorPosition) {
    let Some(line) = editor.content.get(position.line) else {
        return;
    };

    position.char = std::cmp::min(position.char, line.char_len());
    if editor.controls.cursors.is_empty() && !ControlMap::try_multi_cursor(editor) {
        return;
    }
    let cursors = &mut editor.controls.cursors;
    let mut new_cursor = Cursor::default();
    new_cursor.set_position(position);
    match cursors.iter().position(|c| c.get_position() < position) {
        Some(index) => cursors.insert(index, new_cursor),
        None => cursors.push(new_cursor),
    }
}

fn apply_multi_cursor_transaction<F>(editor: &mut Editor, mut callback: F)
where
    F: FnMut(&mut Actions, &mut Lexer, &mut Vec<EditorLine>, &mut Cursor),
{
    let result: Result<(), ()> = transaction::perform_tranasaction(
        &mut editor.actions,
        &mut editor.lexer,
        &mut editor.content,
        |actions, lexer, content| {
            let ControlMap { cursors, .. } = &mut editor.controls;

            // apply first cursor - there is no cursor for offset below
            let Some(first_cursor) = cursors.first_mut() else {
                return Ok(());
            };
            (callback)(actions, lexer, content, first_cursor);

            let mut last_offset_edit = transaction::check_edit_true_count(actions, lexer);
            for cursor_idx in 1..cursors.len() {
                (callback)(actions, lexer, content, &mut cursors[cursor_idx]);

                let current_edit = transaction::check_edit_true_count(actions, lexer);
                while current_edit > last_offset_edit {
                    let edit_offset = transaction::EditOffsetType::parse_edit(actions, last_offset_edit);
                    edit_offset.apply_cursor(cursors.iter_mut().take(cursor_idx))?;
                    last_offset_edit += 1;
                }
            }
            Ok(())
        },
    );

    if result.is_err() {
        // force restore during consolidation of cursors
        editor.controls.cursors.retain(|c| c.max_rows != 0);
    }
}

/// assumption is that from, to represent word location
/// that means all cursors are confirmed to hold the same word
/// and are sorted (consolidated)
fn multi_cursor_word_select(editor: &mut Editor, word: PositionedWord) {
    let (from, ..) = word.range().as_select();
    if let Some(new_range) = word.find_word_inline_after(&editor.content).and_then(|mut iter| iter.next()) {
        let (new_from, new_to) = new_range.as_select();
        let mut new_cursor = Cursor::default();
        editor.cursor.set_position(new_to);
        new_cursor.text_width = editor.cursor.text_width;
        new_cursor.select_set(new_from, new_to);
        editor.controls.cursors.insert(0, new_cursor);
        return;
    }
    let top_content = editor.content.iter().enumerate().take(word.line());
    let content_iter = editor.content.iter().enumerate().skip(word.line() + 1).chain(top_content);
    for new_range in word.iter_word_ranges(content_iter) {
        let (new_from, new_to) = new_range.as_select();
        if new_from > from {
            let mut new_cursor = Cursor::default();
            editor.cursor.set_position(new_to);
            new_cursor.text_width = editor.cursor.text_width;
            new_cursor.select_set(new_from, new_to);
            editor.controls.cursors.insert(0, new_cursor);
            return;
        }
        if editor.controls.cursors.iter().skip(1).any(|c| c.select_get() == Some((new_from, new_to))) {
            continue;
        };

        let mut new_cursor = Cursor::default();
        editor.cursor.set_position(new_to);
        new_cursor.text_width = editor.cursor.text_width;
        new_cursor.select_set(new_from, new_to);
        // sorting will figure out the correct postion
        editor.controls.cursors.push(new_cursor);
        return;
    }

    if let Some(inline_start) = word.find_word_inline_before(&editor.content) {
        for new_range in inline_start {
            let (new_from, new_to) = new_range.as_select();
            if editor.controls.cursors.iter().skip(1).any(|c| c.select_get() == Some((new_from, new_to))) {
                continue;
            };
            let mut new_cursor = Cursor::default();
            editor.cursor.set_position(new_to);
            new_cursor.text_width = editor.cursor.text_width;
            new_cursor.select_set(new_from, new_to);
            // sorting will figure out the correct postion
            editor.controls.cursors.push(new_cursor);
            return;
        }
    }
}

// UTILS

fn sort_cursors(x: &Cursor, y: &Cursor) -> std::cmp::Ordering {
    y.line.cmp(&x.line).then(y.char.cmp(&x.char))
}

fn with_new_line_if_not(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::ControlMap;
    use crate::workspace::Cursor;

    impl ControlMap {
        pub fn mock_cursors(&mut self, cursors: Vec<Cursor>) {
            self.cursors = cursors;
        }

        pub fn mock_update_cursors(&mut self) -> &mut Vec<Cursor> {
            &mut self.cursors
        }
    }
}
