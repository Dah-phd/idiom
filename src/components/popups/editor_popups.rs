use super::{generics::PopupActiveSelector, Button, Popup, PopupSelector};
use crate::components::workspace::CursorPosition;
use crate::events::messages::PopupMessage;
use crate::events::WorkspaceEvent;
use crossterm::event::KeyCode;

pub fn save_all_popup() -> Box<Popup> {
    Box::new(Popup {
        message: "Not all opened editors are saved!".into(),
        message_as_buffer_builder: None,
        title: None,
        buttons: vec![
            Button {
                command: |_| PopupMessage::SaveAndExit,
                name: "Save All (Y)",
                key: Some(vec![KeyCode::Char('y'), KeyCode::Char('Y')]),
            },
            Button {
                command: |_| PopupMessage::Exit,
                name: "Don't save (N)",
                key: Some(vec![KeyCode::Char('n'), KeyCode::Char('N')]),
            },
        ],
        size: Some((40, 4)),
        state: 0,
    })
}

pub fn go_to_line_popup() -> Box<Popup> {
    Box::new(Popup {
        message: String::new(),
        message_as_buffer_builder: Some(|ch| if ch.is_numeric() { Some(ch) } else { None }),
        title: None,
        buttons: vec![Button {
            command: |popup| {
                if let Ok(line) = popup.message.parse::<usize>() {
                    return PopupMessage::UpdateWorkspace(WorkspaceEvent::GoToLine(
                        line.checked_sub(1).unwrap_or_default(),
                    ));
                }
                PopupMessage::Done
            },
            name: "GO",
            key: None,
        }],
        size: Some((30, 4)),
        state: 0,
    })
}

pub fn find_in_editor_popup() -> Box<PopupActiveSelector<(CursorPosition, CursorPosition)>> {
    Box::new(PopupActiveSelector::for_editor(
        |popup| {
            if let Some(select) = popup.next() {
                WorkspaceEvent::GoToSelect { select, should_clear: false }.into()
            } else {
                PopupMessage::None
            }
        },
        |popup, editor_state| {
            if let Some(editor) = editor_state.get_active() {
                editor.find(popup.pattern.as_str(), &mut popup.options);
            }
        },
        Some(|popup| WorkspaceEvent::SelectOpenedFile(popup.pattern.to_owned()).into()),
    ))
}

pub fn replace_in_editor_popup() -> Box<PopupActiveSelector<(CursorPosition, CursorPosition)>> {
    Box::new(PopupActiveSelector::default(
        |popup| {
            if let Some(select) = popup.drain_next() {
                WorkspaceEvent::ReplaceSelect(popup.pattern.to_owned(), select).into()
            } else {
                PopupMessage::Done
            }
        },
        None,
    ))
}

pub fn select_selector(
    options: Vec<((CursorPosition, CursorPosition), String)>,
) -> Box<PopupSelector<((CursorPosition, CursorPosition), String)>> {
    Box::new(PopupSelector {
        options,
        display: |((from, _), line)| format!("({}) {line}", from.line + 1),
        command: |popup| WorkspaceEvent::GoToSelect { select: popup.options[popup.state].0, should_clear: true }.into(),
        state: 0,
        size: None,
    })
}

pub fn editor_selector(options: Vec<String>) -> Box<PopupSelector<String>> {
    Box::new(PopupSelector {
        options,
        display: |editor| editor.to_owned(),
        command: |popup| WorkspaceEvent::ActivateEditor(popup.state).into(),
        state: 0,
        size: None,
    })
}
