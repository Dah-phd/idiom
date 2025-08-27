use super::Actions;
use crate::{
    syntax::{Lexer, SyncCallbacks},
    utils::Offset,
    workspace::{
        actions::{Edit, EditType, EditorLine},
        Cursor, CursorPosition,
    },
};

/// performs multiple actions merged into single edit
pub fn perform_tranasaction<F, R>(
    actions: &mut Actions,
    lexer: &mut Lexer,
    content: &mut Vec<EditorLine>,
    callback: F,
) -> R
where
    F: FnOnce(&mut Actions, &mut Lexer, &mut Vec<EditorLine>) -> R,
{
    let edits_done = std::mem::take(&mut actions.done);
    let stopped_syncs = SyncCallbacks::take(lexer);

    let result = (callback)(actions, lexer, content);

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

    result
}

/// ensures buffer is also pushed
pub fn check_edit_true_count(actions: &mut Actions, lexer: &mut Lexer) -> usize {
    actions.push_buffer(lexer);
    actions.done.len()
}

pub fn get_edit(actions: &Actions, index: usize) -> &EditType {
    &actions.done[index]
}

#[derive(Debug)]
pub struct EditOffset {
    start: CursorPosition,
    line_offset: Offset,
    char_offset: Offset,
}

impl EditOffset {
    fn new(edit: &Edit) -> Self {
        let start = edit.end_position_rev();
        let end = edit.end_position();
        let line_offset = Offset::Pos(end.line) - start.line;
        let char_offset = Offset::Pos(end.char) - start.char;
        Self { start, line_offset, char_offset }
    }

    // errors if position is before cursor
    fn apply_cursor(&self, cursor: &mut Cursor) -> Result<(), ()> {
        if self.start.line > cursor.line {
            return Err(());
        }
        let position = match cursor.line == self.start.line {
            true => {
                if self.start.char > cursor.char {
                    return Err(());
                }
                CursorPosition {
                    line: self.line_offset.offset(cursor.line),
                    char: self.char_offset.offset(cursor.char),
                }
            }
            false => CursorPosition { line: self.line_offset.offset(cursor.line), char: cursor.char },
        };
        cursor.set_position(position);
        Ok(())
    }
}

#[derive(Debug)]
pub enum EditOffsetType {
    Single(EditOffset),
    Multi(Vec<EditOffset>),
}

impl EditOffsetType {
    pub fn new(edit_type: &EditType) -> Self {
        match edit_type {
            EditType::Single(edit) => Self::Single(EditOffset::new(edit)),
            EditType::Multi(edits) => Self::Multi(edits.iter().map(EditOffset::new).collect()),
        }
    }

    pub fn get_from_edit(actions: &Actions, index: usize) -> Self {
        Self::new(get_edit(actions, index))
    }

    pub fn apply_cursor<'a>(&self, cursors: impl Iterator<Item = &'a mut Cursor>) -> Result<(), ()> {
        match self {
            Self::Single(offset) => cursors.map(|cursor| offset.apply_cursor(cursor)).collect(),
            Self::Multi(offsets) => {
                cursors.map(|cursor| offsets.iter().map(|offset| offset.apply_cursor(cursor)).collect()).collect()
            }
        }
    }
}
