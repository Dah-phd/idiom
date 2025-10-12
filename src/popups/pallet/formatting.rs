use crate::{embeded_term::EditorTerminal, global_state::GlobalState, tree::Tree, workspace::Workspace};

pub fn uppercase(_gs: &mut GlobalState, ws: &mut Workspace, _tree: &mut Tree, _term: &mut EditorTerminal) {
    if let Some(editor) = ws.get_active() {
        if editor.cursor.select_is_none() {
            editor.select_word();
        }
        if editor.cursor.select_is_none() {
            return;
        }
        if let Some(clip) = editor.copy() {
            editor.insert_snippet(clip.to_uppercase(), None);
        }
    }
}

pub fn lowercase(_gs: &mut GlobalState, ws: &mut Workspace, _tree: &mut Tree, _term: &mut EditorTerminal) {
    if let Some(editor) = ws.get_active() {
        if editor.cursor.select_is_none() {
            editor.select_word();
        }
        if editor.cursor.select_is_none() {
            return;
        }
        if let Some(clip) = editor.copy() {
            editor.insert_snippet(clip.to_lowercase(), None);
        }
    }
}
