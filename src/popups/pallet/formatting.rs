use crate::{tree::Tree, workspace::Workspace};

pub fn uppercase(ws: &mut Workspace, _tree: &mut Tree) {
    if let Some(editor) = ws.get_active() {
        if editor.cursor.select_is_none() {
            editor.select_token();
        }
        if editor.cursor.select_is_none() {
            return;
        }
        if let Some(clip) = editor.copy() {
            editor.insert_snippet(clip.to_uppercase(), None);
        }
    }
}

pub fn lowercase(ws: &mut Workspace, _tree: &mut Tree) {
    if let Some(editor) = ws.get_active() {
        if editor.cursor.select_is_none() {
            editor.select_token();
        }
        if editor.cursor.select_is_none() {
            return;
        }
        if let Some(clip) = editor.copy() {
            editor.insert_snippet(clip.to_lowercase(), None);
        }
    }
}
