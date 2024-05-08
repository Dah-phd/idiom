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

// transition
pub fn full_rebuild(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    gs.screen_rect.clear(&mut gs.writer)?;
    gs.recalc_draw_size();
    if let Some(line) = gs.footer_area.get_line(0) {
        gs.mode.render(line, gs.theme.accent_style, &mut gs.writer)?;
    }
    workspace.render(gs)?;
    match workspace.get_active() {
        Some(editor) => editor.render(gs),
        None => gs.editor_area.clear(&mut gs.writer),
    }?;
    gs.draw_callback = draw;
    if gs.components.contains(Components::TREE) || !gs.is_insert() {
        gs.draw_callback = draw_with_tree;
        tree.render(gs)?;
    }
    if gs.components.contains(Components::TERM) {
        gs.draw_callback = draw_term
    }
    if gs.components.contains(Components::POPUP) {
        gs.draw_callback = draw_popup;
        gs.render_popup()?;
    }
    Ok(())
}

#[inline]
pub fn draw(gs: &mut GlobalState, workspace: &mut Workspace, _ft: &mut Tree, _t: &mut EditorTerminal) -> Result<()> {
    gs.message.render(gs.theme.accent_style, &mut gs.writer)?;
    workspace.render(gs)?;
    if let Some(editor) = workspace.get_active() {
        return editor.render(gs);
    };
    Ok(())
}

#[inline]
pub fn draw_with_tree(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    gs.writer.hide_cursor()?;
    tree.fast_render(gs)?;
    workspace.render(gs)?;
    gs.message.render(gs.theme.accent_style, &mut gs.writer)?;
    if let Some(editor) = workspace.get_active() {
        return editor.render(gs);
    }
    Ok(())
}

#[inline]
pub fn draw_popup(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    gs.writer.hide_cursor()?;
    gs.message.render(gs.theme.accent_style, &mut gs.writer)?;
    gs.render_popup()
}

#[inline]
pub fn draw_term(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    gs.writer.hide_cursor()?;
    gs.message.render(gs.theme.accent_style, &mut gs.writer)?;
    term.render(gs)
}
