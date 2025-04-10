use std::path::PathBuf;

use crossterm::event::KeyCode;

use super::generic_popup::{CommandButton, Popup};
use super::PopupSelector;
use crate::global_state::{IdiomEvent, PopupMessage};
use crate::workspace::CursorPosition;

pub fn selector_ranges(
    options: Vec<((CursorPosition, CursorPosition), String)>,
) -> Box<PopupSelector<((CursorPosition, CursorPosition), String)>> {
    Box::new(PopupSelector::new(
        options,
        // display: |((from, _), line)| format!("({}) {line}", from.line + 1),
        |((..), line)| line,
        |popup| {
            let (from, to) = popup.options[popup.state.selected].0;
            PopupMessage::ClearEvent(IdiomEvent::GoToSelect { from, to })
        },
        None,
    ))
}

pub fn selector_editors(options: Vec<String>) -> Box<PopupSelector<String>> {
    Box::new(PopupSelector::new(
        options,
        |editor| editor,
        |popup| PopupMessage::ClearEvent(IdiomEvent::ActivateEditor(popup.state.selected)),
        None,
    ))
}

pub fn file_updated(path: PathBuf) -> Popup<IdiomEvent> {
    Popup::new(
        "File updated! (Use cancel/close to do nothing)".into(),
        None,
        Some(path.display().to_string()),
        None,
        vec![
            CommandButton {
                command: |_, _| IdiomEvent::Save,
                name: "Overwrite (S)",
                key: Some(vec![KeyCode::Char('s'), KeyCode::Char('S')]),
            },
            CommandButton {
                command: |_, _| IdiomEvent::Rebase,
                name: "Rebase (L)",
                key: Some(vec![KeyCode::Char('l'), KeyCode::Char('L')]),
            },
        ],
        Some((4, 60)),
    )
}
