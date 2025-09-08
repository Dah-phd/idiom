mod event;
mod methods;

use crate::{
    configs::EditorAction,
    global_state::GlobalState,
    syntax::Lexer,
    workspace::{
        actions::{transaction, Actions},
        Cursor, CursorPosition, Editor, EditorLine,
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
    pub cursors: Vec<Cursor>,
}

impl ControlMap {
    pub fn multi_cursor(editor: &mut Editor) {
        if !editor.file_type.is_code() {
            return;
        }
        editor.controls.cursors.clear();
        editor.controls.cursors.push(editor.cursor.clone());
        editor.controls.multi_cursor_map();
        editor.renderer.multi_cursor();
    }

    pub fn single_cursor(editor: &mut Editor) {
        match editor.controls.cursors.iter().find(|c| c.max_rows != 0) {
            Some(cursor) => editor.cursor.set_cursor(cursor),
            None => editor.cursor.set_position(CursorPosition::default()),
        };
        editor.last_render_at_line = None;
        editor.controls.single_cursor_map();
        editor.renderer.single_cursor();
    }

    pub fn ensure_single_cursor(editor: &mut Editor) {
        if editor.controls.cursors.is_empty() {
            return;
        }
        Self::single_cursor(editor);
    }

    fn multi_cursor_map(&mut self) {
        self.action_map = event::multi_cursor_map;

        self.insert_import = methods::multic_insert_import;
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

pub fn push_multicursor_position(editor: &mut Editor, mut position: CursorPosition) {
    let Some(line) = editor.content.get(position.line) else {
        return;
    };

    position.char = std::cmp::min(position.char, line.char_len());
    if editor.controls.cursors.is_empty() {
        ControlMap::multi_cursor(editor);
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
            let mut index = 0;
            let mut last_edit_idx = 0;
            while let Some(cursor) = editor.controls.cursors.get_mut(index) {
                (callback)(actions, lexer, content, cursor);

                let current_edit_idx = transaction::check_edit_true_count(actions, lexer);
                if current_edit_idx > last_edit_idx && index > 0 {
                    let edit_offset = transaction::EditOffsetType::get_from_edit(actions, current_edit_idx - 1);
                    edit_offset.apply_cursor(editor.controls.cursors.iter_mut().take(index))?;
                };
                last_edit_idx = current_edit_idx;
                index += 1;
            }
            Ok(())
        },
    );

    if result.is_err() {
        // force restore during consolidation of cursors
        editor.controls.cursors.retain(|c| c.max_rows != 0);
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
