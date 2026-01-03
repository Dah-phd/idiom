use super::{GlobalState, Mode};
use crate::{
    embeded_term::EditorTerminal,
    ext_tui::StyleExt,
    tree::Tree,
    workspace::{Workspace, TAB_SELECT},
};
use bitflags::bitflags;
use crossterm::style::ContentStyle;
use idiom_tui::{layout::Line, Backend};

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
        gs.footer.line = msg_line;
        let (mut tree_area, tab_area) = screen.split_horizont_rel(gs.tree_size);
        if let Some(line) = tree_area.next_line() {
            render_logo(line, gs);
        }
        gs.tree_area = tree_area;
        tree.render(gs);
        tab_area
    } else {
        gs.draw_callback = draw;

        let (mode_line, msg_line) = gs.footer_line.clone().split_rel(Mode::len());
        gs.mode.render(mode_line, &mut gs.backend);
        gs.footer.line = msg_line;
        let (tree_area, tab_area) = screen.split_horizont_rel(0);
        gs.tree_area = tree_area;
        tab_area
    };

    (gs.tab_area, gs.editor_area) = screen.split_vertical_rel(1);
    gs.editor_area.left_border();

    workspace.render(gs);
    if let Some(editor) = workspace.get_active() {
        let stats = editor.render(gs);
        gs.footer.render(Some(stats), gs.ui_theme.accent_style(), &mut gs.backend);
    } else {
        gs.footer.render(None, gs.ui_theme.accent_style(), &mut gs.backend);
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
        Some(editor) => {
            let stats = editor.fast_render(gs);
            gs.footer.fast_render(Some(stats), gs.ui_theme.accent_style(), &mut gs.backend);
        }
        None => gs.footer.fast_render(None, gs.ui_theme.accent_style(), &mut gs.backend),
    }
}

pub fn draw_with_tree(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, _term: &mut EditorTerminal) {
    tree.fast_render(gs);
    workspace.fast_render(gs);
    match workspace.get_active() {
        Some(editor) => {
            let stats = editor.fast_render(gs);
            gs.footer.fast_render(Some(stats), gs.ui_theme.accent_style(), &mut gs.backend);
        }
        None => gs.footer.fast_render(None, gs.ui_theme.accent_style(), &mut gs.backend),
    }
}

pub fn draw_term(gs: &mut GlobalState, _workspace: &mut Workspace, _tree: &mut Tree, term: &mut EditorTerminal) {
    gs.backend.save_cursor();
    gs.render_footer(None);
    gs.backend.restore_cursor();
    term.fast_render(gs);
}

fn render_logo(line: Line, gs: &mut GlobalState) {
    if line.width < 10 {
        // should not be reachable
        gs.error(format!("Unexpected tree width: {}", line.width));
        return;
    }
    let style = gs.ui_theme.accent_style();
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
    backend.print_styled(">", ContentStyle::undercurled(None).with_fg(TAB_SELECT));
}
