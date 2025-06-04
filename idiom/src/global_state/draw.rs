use super::{GlobalState, Mode};
use crate::{embeded_term::EditorTerminal, tree::Tree, workspace::Workspace};
use bitflags::bitflags;
use idiom_ui::{
    backend::{Backend, StyleExt},
    layout::{Line, HAVLED_BALANCED_BORDERS},
};

bitflags! {
    /// Workspace and Footer are always drawn
    #[derive(PartialEq, Eq)]
    pub struct Components: u8 {
        const TREE  = 0b0000_0001;
        const TERM  = 0b0000_0010;
    }
}

impl Default for Components {
    fn default() -> Self {
        Components::TREE
    }
}

// transition
pub fn full_rebuild(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
    gs.backend.freeze();

    gs.screen_rect.clear(&mut gs.backend);

    let mut screen = gs.screen_rect;
    gs.footer_line = screen.pop_line();

    let screen = if gs.components.contains(Components::TREE) || gs.is_select() {
        gs.draw_callback = draw_with_tree;

        let (mode_line, msg_line) = gs.footer_line.clone().split_rel(gs.tree_size);
        gs.mode.render(mode_line, &mut gs.backend);
        gs.messages.set_line(msg_line);
        let (mut tree_area, tab_area) = screen.split_horizont_rel(gs.tree_size);
        if let Some(line) = tree_area.next_line() {
            render_logo(line, gs);
        }
        tree_area.right_border().left_border().draw_borders(
            Some(HAVLED_BALANCED_BORDERS),
            Some(gs.theme.accent_background),
            gs.backend(),
        );
        gs.tree_area = tree_area;
        tree.render(gs);
        tab_area
    } else {
        gs.draw_callback = draw;

        let (mode_line, msg_line) = gs.footer_line.clone().split_rel(Mode::len());
        gs.mode.render(mode_line, &mut gs.backend);
        gs.messages.set_line(msg_line);
        let (tree_area, tab_area) = screen.split_horizont_rel(0);
        gs.tree_area = tree_area;
        tab_area
    };

    gs.messages.render(gs.theme.accent_style, &mut gs.backend);
    (gs.tab_area, gs.editor_area) = screen.split_vertical_rel(1);

    workspace.render(gs);
    if let Some(editor) = workspace.get_active() {
        editor.render(gs);
    }

    // term override
    if gs.components.contains(Components::TERM) {
        gs.draw_callback = draw_term;
        term.render(gs);
    }

    gs.backend.unfreeze();
}

pub fn draw(gs: &mut GlobalState, workspace: &mut Workspace, _tree: &mut Tree, _term: &mut EditorTerminal) {
    workspace.fast_render(gs);
    match workspace.get_active() {
        Some(editor) => editor.fast_render(gs),
        None => gs.messages.fast_render(gs.theme.accent_style, &mut gs.backend),
    }
}

pub fn draw_with_tree(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, _term: &mut EditorTerminal) {
    tree.fast_render(gs);
    workspace.fast_render(gs);
    match workspace.get_active() {
        Some(editor) => editor.fast_render(gs),
        None => gs.messages.fast_render(gs.theme.accent_style, &mut gs.backend),
    }
}

pub fn draw_term(gs: &mut GlobalState, _workspace: &mut Workspace, _tree: &mut Tree, term: &mut EditorTerminal) {
    gs.fast_render_message_with_preserved_cursor();
    term.fast_render(gs);
}

fn render_logo(line: Line, gs: &mut GlobalState) {
    if line.width < 10 {
        // should not be reachable
        gs.error(format!("Unexpected tree width: {}", line.width));
        return;
    }
    let style = gs.theme.accent_style;
    let backend = gs.backend();
    let reset_style = backend.get_style();
    backend.set_style(style);
    let pad = line.width - 8;
    let l_pad = pad / 2;
    let r_pad = pad - l_pad;
    backend.go_to(line.row, line.col);
    backend.pad(l_pad);
    backend.print('<');
    backend.set_style(style.with_fg(Mode::insert_color()));
    backend.print("/idiom>");
    backend.pad(r_pad);
    backend.set_style(reset_style);
}
