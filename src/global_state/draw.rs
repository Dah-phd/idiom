use super::{
    controls::{disable_mouse, map_small_rect},
    GlobalState, Mode, MIN_HEIGHT, MIN_WIDTH,
};
use crate::{
    render::{
        backend::{BackendProtocol, StyleExt},
        layout::{Line, HAVLED_BALANCED_BORDERS},
    },
    runner::EditorTerminal,
    tree::Tree,
    workspace::Workspace,
};
use bitflags::bitflags;
use crossterm::style::{Color, ContentStyle};

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
pub fn full_rebuild(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
    gs.screen_rect.clear(&mut gs.writer);

    if gs.screen_rect.width < MIN_WIDTH || gs.screen_rect.height < MIN_HEIGHT {
        gs.draw_callback = draw_too_small_rect;
        gs.key_mapper = map_small_rect;
        gs.mouse_mapper = disable_mouse;
        return;
    }

    let mut tree_area = gs.screen_rect;
    gs.footer_line = tree_area.pop_line();

    if gs.components.contains(Components::TREE) || !gs.is_insert() {
        let (mode_line, msg_line) = gs.footer_line.clone().split_at(gs.tree_size);
        gs.mode.render(mode_line, &mut gs.writer);
        gs.messages.set_line(msg_line);
        gs.draw_callback = draw_with_tree;
        gs.tab_area = tree_area.keep_col(gs.tree_size);
        if let Some(line) = tree_area.next_line() {
            render_logo(line, gs);
        }
        tree_area.top_border().right_border().left_border().draw_borders(
            Some(HAVLED_BALANCED_BORDERS),
            Some(gs.theme.accent_background),
            gs.backend(),
        );
        gs.tree_area = tree_area;
        tree.render(gs);
    } else {
        let (mode_line, msg_line) = gs.footer_line.clone().split_at(Mode::len());
        gs.mode.render(mode_line, &mut gs.writer);
        gs.messages.set_line(msg_line);
        gs.draw_callback = draw;
        gs.tree_area = tree_area;
        gs.tab_area = gs.tree_area.keep_col(0);
    }

    gs.messages.render(gs.theme.accent_style, &mut gs.writer);

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
}

pub fn draw_too_small_rect(
    gs: &mut GlobalState,
    _workspace: &mut Workspace,
    _tree: &mut Tree,
    _term: &mut EditorTerminal,
) {
    let error_text = ["Terminal size too small!", "Press Q or D to exit ..."];
    let style = ContentStyle::bold().with_fg(Color::DarkRed);
    for (line, text) in gs.screen_rect.into_iter().zip(error_text) {
        line.render_centered_styled(text, style, gs.backend());
    }
}

pub fn draw(gs: &mut GlobalState, workspace: &mut Workspace, _tree: &mut Tree, _term: &mut EditorTerminal) {
    workspace.render(gs);
    if let Some(editor) = workspace.get_active() {
        editor.fast_render(gs);
    } else {
        gs.messages.fast_render(gs.theme.accent_style, &mut gs.writer);
    };
}

pub fn draw_with_tree(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, _term: &mut EditorTerminal) {
    tree.fast_render(gs);
    workspace.render(gs);
    if let Some(editor) = workspace.get_active() {
        editor.fast_render(gs);
    } else {
        gs.messages.fast_render(gs.theme.accent_style, &mut gs.writer);
    };
}

pub fn draw_popup(gs: &mut GlobalState, _workspace: &mut Workspace, _tree: &mut Tree, _term: &mut EditorTerminal) {
    gs.messages.fast_render(gs.theme.accent_style, &mut gs.writer);
    gs.popup_render();
}

pub fn draw_term(gs: &mut GlobalState, _workspace: &mut Workspace, _tree: &mut Tree, term: &mut EditorTerminal) {
    gs.messages.fast_render(gs.theme.accent_style, &mut gs.writer);
    term.render(gs);
}

fn render_logo(line: Line, gs: &mut GlobalState) {
    match line.width {
        ..5 => {}
        5..10 => {
            let pad = line.width - 4;
            let l_pad = pad / 2;
            let r_pad = pad - l_pad;
            let backend = gs.backend();
            backend.go_to(line.row, line.col);
            backend.pad(l_pad);
            backend.print('<');
            backend.print_styled("/i>", ContentStyle::fg(Mode::insert_color()));
            backend.pad(r_pad);
        }
        10.. => {
            let pad = line.width - 8;
            let l_pad = pad / 2;
            let r_pad = pad - l_pad;
            let backend = gs.backend();
            backend.go_to(line.row, line.col);
            backend.pad(l_pad);
            backend.print('<');
            backend.print_styled("/idiom>", ContentStyle::fg(Mode::insert_color()));
            backend.pad(r_pad);
        }
    }
}
