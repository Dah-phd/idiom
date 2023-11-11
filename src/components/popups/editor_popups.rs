use super::{generics::PopupActiveSelector, Button, Popup, PopupSelector};
use crate::{components::workspace::Select, events::messages::PopupMessage};
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
                    return PopupMessage::GoToLine(line.checked_sub(1).unwrap_or_default());
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

pub fn rename_var_popup() -> Box<Popup> {
    Box::new(Popup {
        message: String::new(),
        title: None,
        message_as_buffer_builder: Some(Some),
        buttons: vec![Button {
            command: |popup| PopupMessage::Rename(popup.message.to_owned()),
            name: "Rename",
            key: None,
        }],
        size: Some((50, 4)),
        state: 0,
    })
}

pub fn find_in_editor_popup() -> Box<PopupActiveSelector<Select>> {
    Box::new(PopupActiveSelector::for_editor(
        |popup| {
            if let Some(select) = popup.next() {
                PopupMessage::GoToSelect { select, should_clear: false }
            } else {
                PopupMessage::None
            }
        },
        |popup, editor_state| {
            if let Some(editor) = editor_state.get_active() {
                editor.find(popup.pattern.as_str(), &mut popup.options);
            }
        },
        Some(|popup| PopupMessage::SelectOpenedFile(popup.pattern.to_owned())),
    ))
}

pub fn replace_in_editor_popup() -> Box<PopupActiveSelector<Select>> {
    Box::new(PopupActiveSelector::default(
        |popup| {
            if let Some(select) = popup.drain_next() {
                PopupMessage::ReplaceSelect(popup.pattern.to_owned(), select)
            } else {
                PopupMessage::Done
            }
        },
        None,
    ))
}

pub fn select_selector(options: Vec<(Select, String)>) -> Box<PopupSelector<(Select, String)>> {
    Box::new(PopupSelector {
        options,
        display: |(select, line)| match select {
            Select::Range(from, ..) => format!("({}) {line}", from.line + 1),
            Select::None => line.to_owned(),
        },
        command: |popup| PopupMessage::GoToSelect { select: popup.options[popup.state].0, should_clear: true },
        state: 0,
        size: None,
    })
}

pub fn editor_selector(options: Vec<String>) -> Box<PopupSelector<String>> {
    Box::new(PopupSelector {
        options,
        display: |editor| editor.to_owned(),
        command: |popup| PopupMessage::ActivateEditor(popup.state),
        state: 0,
        size: None,
    })
}
