use super::Actions;
use crate::{
    actions::{Action, Edit, EditorLine},
    cursor::{Cursor, CursorPosition},
    syntax::{Lexer, SyncCallbacks},
    utils::Offset,
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
            Action::Single(edit) => edits.push(edit),
            Action::Multi(mutli) => edits.extend(mutli),
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

pub fn undo_multi_cursor(
    actions: &mut Actions,
    content: &mut Vec<EditorLine>,
    lexer: &mut Lexer,
    text_width: usize,
) -> Option<Vec<Cursor>> {
    actions.push_buffer(lexer);
    let action = actions.done.pop()?;
    let cursors = match &action {
        Action::Single(edit) => {
            let (position, select) = edit.apply_rev(content);
            let mut cursor = Cursor::default();
            cursor.text_width = text_width;
            match select {
                Some((from, to)) => cursor.select_set(from, to),
                None => cursor.set_position(position),
            }
            vec![cursor]
        }
        Action::Multi(edits) => {
            let cursors = edits
                .iter()
                .rev()
                .map(|edit| {
                    let (position, select) = edit.apply_rev(content);
                    let mut cursor = Cursor::default();
                    cursor.text_width = text_width;
                    match select {
                        Some((from, to)) => cursor.select_set(from, to),
                        None => {
                            cursor.select_drop();
                            cursor.set_position(position)
                        }
                    }
                    cursor
                })
                .collect();
            cursors
        }
    };
    lexer.sync_rev(&action, content);
    actions.undone.push(action);
    Some(cursors)
}

pub fn redo_multi_cursor(
    actions: &mut Actions,
    content: &mut Vec<EditorLine>,
    lexer: &mut Lexer,
    text_width: usize,
) -> Option<Vec<Cursor>> {
    actions.push_buffer(lexer);
    let action = actions.done.pop()?;
    let cursors = match &action {
        Action::Single(edit) => {
            let (position, select) = edit.apply(content);
            let mut cursor = Cursor::default();
            cursor.text_width = text_width;
            match select {
                Some((from, to)) => cursor.select_set(from, to),
                None => cursor.set_position(position),
            }
            vec![cursor]
        }
        Action::Multi(edits) => {
            let cursors = edits
                .iter()
                .map(|edit| {
                    let (position, select) = edit.apply(content);
                    let mut cursor = Cursor::default();
                    cursor.text_width = text_width;
                    match select {
                        Some((from, to)) => cursor.select_set(from, to),
                        None => {
                            cursor.select_drop();
                            cursor.set_position(position)
                        }
                    }
                    cursor
                })
                .collect();
            cursors
        }
    };
    lexer.sync(&action, content);
    actions.undone.push(action);
    Some(cursors)
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
    pub fn new(edit_type: &Action) -> Self {
        match edit_type {
            Action::Single(edit) => Self::Single(EditOffset::new(edit)),
            Action::Multi(edits) => Self::Multi(edits.iter().map(EditOffset::new).collect()),
        }
    }

    pub fn parse_edit(actions: &Actions, index: usize) -> Self {
        Self::new(&actions.done[index])
    }

    pub fn apply_cursor<'a>(&self, mut cursors: impl Iterator<Item = &'a mut Cursor>) -> Result<(), ()> {
        match self {
            Self::Single(offset) => cursors.try_for_each(|cursor| offset.apply_cursor(cursor)),
            Self::Multi(offsets) => {
                cursors.try_for_each(|cursor| offsets.iter().try_for_each(|offset| offset.apply_cursor(cursor)))
            }
        }
    }
}
