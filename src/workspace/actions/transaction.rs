use super::Actions;
use crate::{
    syntax::{Lexer, SyncCallbacks},
    utils::Offset,
    workspace::{
        actions::{Edit, EditType, EditorLine},
        Cursor, CursorPosition,
    },
};
use std::cmp::Ordering;

/// performs multiple actions merged into single edit
pub fn perform_tranasaction<F>(actions: &mut Actions, lexer: &mut Lexer, content: &mut Vec<EditorLine>, callback: F)
where
    F: FnOnce(&mut Actions, &mut Lexer, &mut Vec<EditorLine>),
{
    let edits_done = std::mem::take(&mut actions.done);
    let stopped_syncs = SyncCallbacks::take(lexer);

    (callback)(actions, lexer, content);

    // ensure buffer in pushed into the vec of edits
    actions.push_buffer(lexer);

    stopped_syncs.set_in(lexer);
    let transaction = std::mem::replace(&mut actions.done, edits_done);

    let mut edits = vec![];
    for edit in transaction {
        match edit {
            EditType::Single(edit) => edits.push(edit),
            EditType::Multi(mutli) => edits.extend(mutli),
        }
    }
    if !edits.is_empty() {
        actions.push_done(edits, lexer, content);
    }
}

pub fn check_count(actions: &Actions) -> usize {
    actions.done.len()
}

pub fn get_edit(actions: &Actions, index: usize) -> &EditType {
    &actions.done[index]
}

pub fn offset_cursor(edit: &EditType, cursor: &mut Cursor) {
    match edit {
        EditType::Single(edit) => {
            offset_cursor_per_edit(edit, cursor);
        }
        EditType::Multi(edits) => {
            for edit in edits {
                offset_cursor_per_edit(edit, cursor);
            }
        }
    }
}

fn offset_cursor_per_edit(edit: &Edit, cursor: &mut Cursor) {
    let start = edit.end_position_rev();
    let end = edit.end_position();
    let line_offset = Offset::Pos(end.line) - start.line;
    let position = if cursor.line == start.line {
        let char_offset = Offset::Pos(end.char) - start.char;
        CursorPosition { line: line_offset.offset(cursor.line), char: char_offset.offset(cursor.char) }
    } else {
        CursorPosition { line: line_offset.offset(cursor.line), char: cursor.char }
    };
    cursor.set_position(position);
}
