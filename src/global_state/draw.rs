use super::{GlobalState, Mode};
use crate::{
    render::{
        backend::{BackendProtocol, Style},
        layout::{Line, HAVLED_BALANCED_BORDERS},
    },
    runner::EditorTerminal,
    tree::Tree,
    workspace::Workspace,
};
use bitflags::bitflags;
use std::io::{Result, Write};

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
    gs.screen_rect.clear(&mut gs.writer);
    let mut tree_area = gs.screen_rect;
    gs.footer_area = tree_area.splitoff_rows(1);
    if let Some(mut line) = gs.footer_area.get_line(0) {
        gs.mode.render(line.clone(), gs.theme.accent_style, &mut gs.writer);
        line += Mode::len();
        gs.messages.set_line(line);
    };
    gs.messages.render(gs.theme.accent_style, &mut gs.writer);

    if gs.components.contains(Components::TREE) || !gs.is_insert() {
        gs.draw_callback = draw_with_tree;
        gs.tab_area = tree_area.keep_col((gs.tree_size * gs.screen_rect.width) / 100);
        if let Some(line) = tree_area.next_line() {
            render_logo(line, gs);
        }
        tree_area.top_border().right_border().draw_borders(
            Some(HAVLED_BALANCED_BORDERS),
            Some(gs.theme.accent_background),
            gs.backend(),
        );
        gs.tree_area = tree_area;
        tree.render(gs);
    } else {
        gs.draw_callback = draw;
        gs.tree_area = tree_area;
        gs.tab_area = gs.tree_area.keep_col(0);
    }

    gs.editor_area = gs.tab_area.keep_rows(1);
    workspace.render(gs);
    if let Some(editor) = workspace.get_active() {
        editor.render(gs);
    }

    // term override
    if gs.components.contains(Components::TERM) {
        gs.draw_callback = draw_term;
        term.render(gs);
    }
    // popup override
    if gs.components.contains(Components::POPUP) {
        gs.draw_callback = draw_popup;
        gs.popup_render();
    }

    gs.writer.flush()
}

pub fn draw(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    _tree: &mut Tree,
    _term: &mut EditorTerminal,
) -> Result<()> {
    workspace.render(gs);
    if let Some(editor) = workspace.get_active() {
        editor.fast_render(gs);
    } else {
        gs.messages.fast_render(gs.theme.accent_style, &mut gs.writer);
    };
    gs.writer.flush()
}

pub fn draw_with_tree(
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    tree: &mut Tree,
    _term: &mut EditorTerminal,
) -> Result<()> {
    tree.fast_render(gs);
    workspace.render(gs);
    if let Some(editor) = workspace.get_active() {
        editor.fast_render(gs);
    } else {
        gs.messages.fast_render(gs.theme.accent_style, &mut gs.writer);
    };
    gs.writer.flush()
}

pub fn draw_popup(
    gs: &mut GlobalState,
    _workspace: &mut Workspace,
    _tree: &mut Tree,
    _term: &mut EditorTerminal,
) -> Result<()> {
    gs.messages.fast_render(gs.theme.accent_style, &mut gs.writer);
    gs.popup_render();
    gs.writer.flush()
}

pub fn draw_term(
    gs: &mut GlobalState,
    _workspace: &mut Workspace,
    _tree: &mut Tree,
    term: &mut EditorTerminal,
) -> Result<()> {
    gs.messages.fast_render(gs.theme.accent_style, &mut gs.writer);
    term.render(gs);
    gs.writer.flush()
}

fn render_logo(line: Line, gs: &mut GlobalState) {
    if line.width < 9 {
        return;
    };
    let pad = line.width - 9;
    if pad == 0 {
        return;
    }
    let l_pad = pad / 2;
    let r_pad = pad - l_pad;
    let backend = gs.backend();
    backend.go_to(line.row, line.col);
    backend.pad(l_pad);
    backend.print('<');
    backend.print_styled("/idiom>", Style::fg(Mode::insert_color()));
    backend.pad(r_pad);
}
