use crate::{embeded_term::EditorTerminal, global_state::GlobalState, tree::Tree, workspace::Workspace};

/// if terminal is open palled cannot be opened
pub fn open_embeded_terminal(gs: &mut GlobalState, _ws: &mut Workspace, _tree: &mut Tree, term: &mut EditorTerminal) {
    gs.toggle_terminal(term);
}

pub fn select_lsp(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
    crate::popups::popup_lsp_select::SelectorLSP::run(gs, ws, tree, term);
}
