use ratatui::Frame;

use crate::{footer::Footer, runner::EditorTerminal, tree::Tree, workspace::Workspace};

use super::GlobalState;

pub fn full_draw(
    gs: &mut GlobalState,
    frame: &mut Frame,
    workspace: &mut Workspace,
    file_tree: &mut Tree,
    footer: &mut Footer,
    tmux: &mut EditorTerminal,
) {
    file_tree.render(frame, gs);
    footer.render(frame, gs, workspace.get_stats());
    workspace.render(frame, gs);
    tmux.render(frame, gs.editor_area);
    gs.render_popup_if_exists(frame);
}

pub fn inactive_tmux(
    gs: &mut GlobalState,
    frame: &mut Frame,
    workspace: &mut Workspace,
    file_tree: &mut Tree,
    footer: &mut Footer,
    _t: &mut EditorTerminal,
) {
    file_tree.render(frame, gs);
    footer.render(frame, gs, workspace.get_stats());
    workspace.render(frame, gs);
    gs.render_popup_if_exists(frame);
}

pub fn inactive_tree(
    gs: &mut GlobalState,
    frame: &mut Frame,
    workspace: &mut Workspace,
    _ft: &mut Tree,
    footer: &mut Footer,
    tmux: &mut EditorTerminal,
) {
    footer.render(frame, gs, workspace.get_stats());
    workspace.render(frame, gs);
    tmux.render(frame, gs.editor_area);
    gs.render_popup_if_exists(frame);
}

pub fn inactive_tree_and_tmux(
    gs: &mut GlobalState,
    frame: &mut Frame,
    workspace: &mut Workspace,
    _ft: &mut Tree,
    footer: &mut Footer,
    _t: &mut EditorTerminal,
) {
    footer.render(frame, gs, workspace.get_stats());
    workspace.render(frame, gs);
    gs.render_popup_if_exists(frame);
}
