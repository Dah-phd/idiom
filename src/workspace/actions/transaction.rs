use super::Actions;
use crate::{
    syntax::{Lexer, SyncCallbacks},
    workspace::actions::{EditType, EditorLine},
};

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
