use crate::{embeded_term::EditorTerminal, global_state::GlobalState, tree::Tree, workspace::Workspace};

/// if terminal is open palled cannot be opened
pub fn open_embeded_terminal(gs: &mut GlobalState, _ws: &mut Workspace, _tree: &mut Tree, term: &mut EditorTerminal) {
    gs.toggle_terminal(term);
}
