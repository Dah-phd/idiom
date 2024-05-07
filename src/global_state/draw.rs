use crate::{global_state::GlobalState, runner::EditorTerminal, tree::Tree, workspace::Workspace};
use bitflags::bitflags;
use std::io::Result;

bitflags! {
    /// Workspace and Footer are always drawn
    #[derive(PartialEq, Eq)]
    pub struct Components: u8 {
        const TREE  = 0b0000_0001;
        const POPUP = 0b0000_0010;
        const TERM  = 0b0000_0100;
    }
}

impl Default for Components {
    fn default() -> Self {
        Components::TREE
    }
}

// DRAW callbacks
pub fn draw(gs: &mut GlobalState, workspace: &mut Workspace, _ft: &mut Tree, _t: &mut EditorTerminal) -> Result<()> {
    workspace.render(gs)?;
    if let Some(editor) = workspace.get_active() {
        editor.render(gs)?;
    }
    Ok(())
}

pub fn draw_with_tree(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    _t: &mut EditorTerminal,
) -> Result<()> {
    tree.render(gs)?;
    workspace.render(gs)?;
    if let Some(editor) = workspace.get_active() {
        editor.render(gs)?;
    }
    Ok(())
}

pub fn draw_with_popup(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    workspace.render(gs)?;
    if let Some(editor) = workspace.get_active() {
        editor.render(gs)?;
    }
    gs.render_popup_if_exists()
}

pub fn draw_with_term(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    workspace.render(gs)?;
    if let Some(editor) = workspace.get_active() {
        editor.render(gs)?;
    }
    term.render(gs)
}

pub fn draw_with_term_and_popup(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    workspace.render(gs)?;
    if let Some(editor) = workspace.get_active() {
        editor.render(gs)?;
    }
    term.render(gs)?;
    gs.render_popup_if_exists()
}

pub fn draw_with_tree_and_popup(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    tree.render(gs)?;
    workspace.render(gs)?;
    if let Some(editor) = workspace.get_active() {
        editor.render(gs)?;
    }
    term.render(gs)
}

pub fn draw_with_tree_and_term(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    tree.render(gs)?;
    workspace.render(gs)?;
    if let Some(editor) = workspace.get_active() {
        editor.render(gs)?;
    }
    Ok(())
}

pub fn draw_full(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    tree.render(gs)?;
    workspace.render(gs)?;
    if let Some(editor) = workspace.get_active() {
        editor.render(gs)?;
    }
    term.render(gs)?;
    gs.render_popup_if_exists()
}
