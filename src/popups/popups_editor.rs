use super::{Popup, PopupSelector};
use crate::global_state::WorkspaceEvent;
use crate::widgests::Button;
use crate::workspace::CursorPosition;
use crossterm::event::KeyCode;

pub fn save_all_popup() -> Box<Popup> {
    Box::new(Popup {
        message: "Not all opened editors are saved!".into(),
        message_as_buffer_builder: None,
        title: None,
        buttons: vec![
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
        size: Some((40, 4)),
        state: 0,
    })
}

pub fn selector_ranges(
    options: Vec<((CursorPosition, CursorPosition), String)>,
) -> Box<PopupSelector<((CursorPosition, CursorPosition), String)>> {
    Box::new(PopupSelector {
        options,
        display: |((from, _), line)| format!("({}) {line}", from.line + 1),
        command: |popup| WorkspaceEvent::GoToSelect { select: popup.options[popup.state].0, clear_popup: true }.into(),
        state: 0,
        size: None,
    })
}

pub fn selector_editors(options: Vec<String>) -> Box<PopupSelector<String>> {
    Box::new(PopupSelector {
        options,
        display: |editor| editor.to_owned(),
        command: |popup| WorkspaceEvent::ActivateEditor(popup.state).into(),
        state: 0,
        size: None,
    })
}
