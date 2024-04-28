use std::rc::Rc;

use bitflags::bitflags;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

const RECT_CONSTRAINT: [Constraint; 2] = [Constraint::Length(1), Constraint::Percentage(100)];

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

// LAYOUTS

pub fn layour_workspace_footer(screen: Rect) -> Rc<[Rect]> {
    Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(screen.height.saturating_sub(1)),
            Constraint::Length(1),
        ],
    )
    .split(screen)
}

pub fn layout_tree(screen: Rect, size: u16) -> Rc<[Rect]> {
    Layout::new(Direction::Horizontal, [Constraint::Percentage(size), Constraint::Min(2)]).split(screen)
}

pub fn layot_tabs_editor(screen: Rect) -> Rc<[Rect]> {
    Layout::new(Direction::Vertical, RECT_CONSTRAINT).split(screen)
}
