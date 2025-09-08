mod event;
pub use event::{multi_cursor_map, single_cursor_map};

use crate::{
    configs::EditorAction,
    global_state::GlobalState,
    syntax::Lexer,
    workspace::{
        actions::{transaction, Actions},
        Cursor, CursorPosition, Editor, EditorLine,
    },
};

/// holds controls and manages cursor type
/// if switched to multicursor will swap callbacks to multi cursor
pub struct ControlMap {
    pub action_map: fn(&mut Editor, EditorAction, gs: &mut GlobalState) -> bool,
    pub cursors: Vec<Cursor>,
}

impl Default for ControlMap {
    fn default() -> Self {
        Self { action_map: single_cursor_map, cursors: Vec::default() }
    }
}

pub fn consolidate_cursors_per_line(editor: &mut Editor) {
    let mut idx = 1;
    editor.multi_positions.sort_by(sort_cursors);

    while idx < editor.multi_positions.len() {
        unsafe {
            let [cursor, other] = editor.multi_positions.get_disjoint_unchecked_mut([idx - 1, idx]);
            if cursor.line == other.line {
                cursor.max_rows = std::cmp::max(cursor.max_rows, other.max_rows);
                editor.multi_positions.remove(idx);
            } else {
                idx += 1;
            }
        }
    }
    if editor.multi_positions.len() < 2 {
        restore_single_cursor_mode(editor);
    }
}

pub fn filter_multi_cursors_per_line_if_no_select(editor: &Editor) -> Vec<Cursor> {
    let mut filtered = vec![];
    let mut index = 0;
    loop {
        let Some(mut cursor) = editor.multi_positions.get(index).cloned() else {
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
            while let Some(next_cursor) = editor.multi_positions.get(index) {
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

    editor.multi_positions.sort_by(sort_cursors);

    while idx < editor.multi_positions.len() {
        unsafe {
            let [cursor, other] = editor.multi_positions.get_disjoint_unchecked_mut([idx - 1, idx]);
            if cursor.merge_if_intersect(other) {
                cursor.max_rows = std::cmp::max(cursor.max_rows, other.max_rows);
                editor.multi_positions.remove(idx);
            } else {
                idx += 1;
            }
        }
    }
    if editor.multi_positions.len() < 2 {
        restore_single_cursor_mode(editor);
    }
}

pub fn ensure_single_cursor(editor: &mut Editor) {
    if editor.multi_positions.is_empty() {
        return;
    }
    restore_single_cursor_mode(editor);
}

pub fn restore_single_cursor_mode(editor: &mut Editor) {
    match editor.multi_positions.iter().find(|c| c.max_rows != 0) {
        Some(cursor) => editor.cursor.set_cursor(cursor),
        None => editor.cursor.set_position(CursorPosition::default()),
    };
    editor.last_render_at_line = None;
    editor.controls.action_map = single_cursor_map;
    editor.multi_positions.clear();
    editor.renderer.single_cursor();
}

pub fn enable_multi_cursor_mode(editor: &mut Editor) {
    if !editor.file_type.is_code() {
        return;
    }
    editor.multi_positions.clear();
    editor.multi_positions.push(editor.cursor.clone());
    editor.controls.action_map = multi_cursor_map;
    editor.renderer.multi_cursor();
}

pub fn push_multicursor_position(editor: &mut Editor, mut position: CursorPosition) {
    let Some(line) = editor.content.get(position.line) else {
        return;
    };
    position.char = std::cmp::min(position.char, line.char_len());
    if editor.multi_positions.is_empty() {
        enable_multi_cursor_mode(editor);
    }
    let mut new_cursor = Cursor::default();
    new_cursor.set_position(position);
    match editor.multi_positions.iter().position(|c| c.get_position() < position) {
        Some(index) => editor.multi_positions.insert(index, new_cursor),
        None => editor.multi_positions.push(new_cursor),
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
            while let Some(cursor) = editor.multi_positions.get_mut(index) {
                (callback)(actions, lexer, content, cursor);

                let current_edit_idx = transaction::check_edit_true_count(actions, lexer);
                if current_edit_idx > last_edit_idx && index > 0 {
                    let edit_offset = transaction::EditOffsetType::get_from_edit(actions, current_edit_idx - 1);
                    edit_offset.apply_cursor(editor.multi_positions.iter_mut().take(index))?;
                };
                last_edit_idx = current_edit_idx;
                index += 1;
            }
            Ok(())
        },
    );

    if result.is_err() {
        // force restore during consolidation of cursors
        editor.multi_positions.retain(|c| c.max_rows != 0);
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
