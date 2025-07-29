use crate::{embeded_term::EditorTerminal, global_state::GlobalState, tree::Tree, workspace::Workspace};

/// if terminal is open palled cannot be opened
pub fn open_embeded_terminal(gs: &mut GlobalState, _ws: &mut Workspace, _tree: &mut Tree, term: &mut EditorTerminal) {
    gs.toggle_terminal(term);
}

pub fn set_lsp(gs: &mut GlobalState, _ws: &mut Workspace, _tree: &mut Tree, _term: &mut EditorTerminal) {
    gs.event.push(crate::global_state::IdiomEvent::SetLSP(crate::configs::FileType::Shell));
}
