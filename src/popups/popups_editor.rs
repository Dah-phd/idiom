use super::{Popup, PopupSelector};
use crate::global_state::WorkspaceEvent;
use crate::render::Button;
use crate::workspace::CursorPosition;
use crossterm::event::KeyCode;

pub fn save_all_popup() -> Box<Popup> {
    Box::new(Popup::new(
        "Not all opened editors are saved!".into(),
        None,
        None,
        vec![
            Button {
                command: |_| WorkspaceEvent::SaveAndExit.into(),
                name: "Save All (Y)",
                key: Some(vec![KeyCode::Char('y'), KeyCode::Char('Y')]),
            },
            Button {
                command: |_| WorkspaceEvent::Exit.into(),
                name: "Don't save (N)",
                key: Some(vec![KeyCode::Char('n'), KeyCode::Char('N')]),
            },
        ],
        Some((4, 40)),
    ))
}

pub fn selector_ranges(
    options: Vec<((CursorPosition, CursorPosition), String)>,
) -> Box<PopupSelector<((CursorPosition, CursorPosition), String)>> {
    Box::new(PopupSelector::new(
        options,
        // display: |((from, _), line)| format!("({}) {line}", from.line + 1),
        |((..), line)| line,
        |popup| WorkspaceEvent::GoToSelect { select: popup.options[popup.state.selected].0, clear_popup: true }.into(),
        None,
    ))
}

pub fn selector_editors(options: Vec<String>) -> Box<PopupSelector<String>> {
    Box::new(PopupSelector::new(
        options,
        |editor| editor,
        |popup| WorkspaceEvent::ActivateEditor(popup.state.selected).into(),
        None,
    ))
}
