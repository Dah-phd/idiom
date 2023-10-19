use super::{generics::PopupActiveSelector, Button, Popup, PopupSelector};
use crate::{components::editor::Select, configs::PopupMessage};
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

pub fn find_in_editor_popup_2() -> Box<Popup> {
    Box::new(Popup {
        message: String::new(),
        title: Some("Search opened file".to_owned()),
        message_as_buffer_builder: Some(Some),
        buttons: vec![Button {
            command: |popup| PopupMessage::SelectOpenedFile(popup.message.to_owned()),
            name: "Search",
            key: None,
        }],
        size: Some((40, 4)),
        state: 0,
    })
}

pub fn find_in_editor_popup() -> Box<PopupActiveSelector<Select>> {
    Box::new(PopupActiveSelector::for_editor(
        |popup| {
            if let Some(select) = popup.next() {
                PopupMessage::GoToSelect(select)
            } else {
                PopupMessage::None
            }
        },
        |popup, editor_state| {
            if let Some(editor) = editor_state.get_active() {
                editor.find(popup.pattern.as_str(), &mut popup.options)
            }
        },
    ))
}

pub fn select_line_popup(options: Vec<(usize, String)>) -> Box<PopupSelector<(usize, String)>> {
    Box::new(PopupSelector {
        options,
        display: |(_, line)| line.to_owned(),
        command: |popup| PopupMessage::GoToLine(popup.options[popup.state].0),
        state: 0,
        size: None,
    })
}

pub fn select_editor_popup(options: Vec<String>) -> Box<PopupSelector<String>> {
    Box::new(PopupSelector {
        options,
        display: |editor| editor.to_owned(),
        command: |popup| PopupMessage::ActivateEditor(popup.state),
        state: 0,
        size: None,
    })
}
