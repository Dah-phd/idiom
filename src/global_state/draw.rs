use std::rc::Rc;

use crate::{footer::Footer, runner::EditorTerminal, tree::Tree, workspace::Workspace};
use bitflags::bitflags;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

const RECT_CONSTRAINT: [Constraint; 2] = [Constraint::Length(1), Constraint::Percentage(100)];

use super::GlobalState;

bitflags! {
    pub struct Components: u8 {
        const EDITOR = 0b0000_0001;
        const TREE   = 0b0000_0010;
        const TERM   = 0b0000_0100;
        const POPUP  = 0b0000_1000;
    }
}

impl Default for Components {
    fn default() -> Self {
        Components::EDITOR | Components::TREE
    }
}

// DRAW callbacks

pub fn full_draw(
    gs: &mut GlobalState,
    frame: &mut Frame,
    workspace: &mut Workspace,
    tree: &mut Tree,
    footer: &mut Footer,
    tmux: &mut EditorTerminal,
) {
    tree.render(frame, gs);
    footer.render(frame, gs, workspace.get_stats());
    workspace.render(frame, gs);
    tmux.render(frame, gs.editor_area);
    gs.render_popup_if_exists(frame);
}

pub fn inactive_tmux(
    gs: &mut GlobalState,
    frame: &mut Frame,
    workspace: &mut Workspace,
    tree: &mut Tree,
    footer: &mut Footer,
    _t: &mut EditorTerminal,
) {
    tree.render(frame, gs);
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

// LAYOUTS

pub fn layour_workspace_footer(screen: Rect) -> Rc<[Rect]> {
    Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(screen.height.saturating_sub(2)),
            Constraint::Length(2),
        ],
    )
    .split(screen)
}

pub fn layout_tree(screen: Rect, size: u16) -> Rc<[Rect]> {
    Layout::new(Direction::Horizontal, [Constraint::Percentage(size), Constraint::Min(2)]).split(screen)
}

pub fn layot_tabs_editor(area: Rect) -> Rc<[Rect]> {
    Layout::new(Direction::Vertical, RECT_CONSTRAINT).split(area)
}
