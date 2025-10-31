use crate::{
    embeded_term::EditorTerminal,
    global_state::GlobalState,
    tree::Tree,
    workspace::{utils::copy_content, Workspace},
};

pub fn uppercase(_gs: &mut GlobalState, ws: &mut Workspace, _tree: &mut Tree, _term: &mut EditorTerminal) {
    let Some(editor) = ws.get_active() else { return };
    editor.apply(|actions, lexer, content, cursor| {
        if cursor.select_is_none() {
            cursor.select_word(content);
        }
        let Some((from, to)) = cursor.select_get() else { return };
        let clip = copy_content(from, to, content);
        actions.insert_snippet(cursor, clip.to_uppercase(), None, content, lexer);
    });
}

pub fn lowercase(_gs: &mut GlobalState, ws: &mut Workspace, _tree: &mut Tree, _term: &mut EditorTerminal) {
    let Some(editor) = ws.get_active() else { return };
    editor.apply(|actions, lexer, content, cursor| {
        if cursor.select_is_none() {
            cursor.select_word(content);
        }
        let Some((from, to)) = cursor.select_get() else { return };
        let clip = copy_content(from, to, content);
        actions.insert_snippet(cursor, clip.to_lowercase(), None, content, lexer);
    });
}
