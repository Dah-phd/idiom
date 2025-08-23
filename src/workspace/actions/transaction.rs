use super::Actions;
use crate::{
    syntax::{Lexer, SyncCallbacks},
    workspace::actions::{EditType, EditorLine},
};

pub struct Transaction {
    sync: SyncCallbacks,
    edits_done: Vec<EditType>,
}

impl Transaction {
    pub fn new(edits_done: Vec<EditType>, lexer: &mut Lexer) -> Self {
        Self { sync: SyncCallbacks::take(lexer), edits_done }
    }

    pub fn finish(self, actions: &mut Actions, lexer: &mut Lexer, content: &[EditorLine]) {
        actions.push_buffer(lexer);
        let Self { edits_done, sync } = self;
        sync.set(lexer);
        let transaction = std::mem::replace(&mut actions.done, edits_done);
        let mut edits = vec![];
        for edit in transaction {
            match edit {
                EditType::Single(edit) => edits.push(edit),
                EditType::Multi(mutli) => edits.extend(mutli),
            }
        }
        actions.push_done(edits, lexer, content);
    }
}
