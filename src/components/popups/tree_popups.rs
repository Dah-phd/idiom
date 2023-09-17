use std::path::PathBuf;

use super::{Button, Popup, PopupSelector};
use crate::configs::PopupMessage;

pub fn create_file_popup(path: String) -> Popup {
    let mut buttons = vec![Button {
        command: |popup| PopupMessage::CreateFileOrFolder(popup.message.to_owned()),
        name: "Create".to_owned(),
        key: None,
    }];
    if path != "./" {
        buttons.push(Button {
            command: |popup| PopupMessage::CreateFileOrFolderBase(popup.message.to_owned()),
            name: "Create in ./".to_owned(),
            key: None,
        })
    }
    Popup {
        message: String::new(),
        message_as_buffer_builder: Some(Some),
        title: Some(format!("New in {}", path)),
        buttons,
        size: Some((40, 4)),
        state: 0,
    }
}

pub fn rename_file_popup(path: String) -> Popup {
    Popup {
        message: String::new(),
        message_as_buffer_builder: Some(Some),
        title: None,
        buttons: vec![Button {
            command: |popup| PopupMessage::RenameFile(popup.message.to_owned()),
            name: format!("Rename: {path}"),
            key: None,
        }],
        size: Some((40, 4)),
        state: 0,
    }
}

pub fn find_in_tree_popup() -> Popup {
    Popup {
        message: String::new(),
        message_as_buffer_builder: Some(Some),
        title: Some("Find in tree".to_owned()),
        buttons: vec![
            Button {
                command: |popup| PopupMessage::SelectPath(popup.message.to_owned()),
                name: "Paths".to_owned(),
                key: None,
            },
            Button {
                command: |popup| PopupMessage::SelectTreeFiles(popup.message.to_owned()),
                name: "Files".to_owned(),
                key: None,
            },
            Button {
                command: |popup| PopupMessage::SelectPathFull(popup.message.to_owned()),
                name: "All paths".to_owned(),
                key: None,
            },
            Button {
                command: |popup| PopupMessage::SelectTreeFilesFull(popup.message.to_owned()),
                name: "All files".to_owned(),
                key: None,
            },
        ],
        size: Some((65, 4)),
        state: 0,
    }
}

pub fn select_tree_file_popup(options: Vec<(PathBuf, String, usize)>) -> PopupSelector<(PathBuf, String, usize)> {
    PopupSelector {
        options,
        display: |(path, text, _)| format!("{}\n    {text}", path.display()),
        command: |popup| {
            if let Some((path, _, idx)) = popup.options.get(popup.state) {
                return PopupMessage::Open((path.clone(), *idx));
            }
            PopupMessage::Done
        },
        size: None,
        state: 0,
    }
}

pub fn select_file_popup(options: Vec<PathBuf>) -> PopupSelector<PathBuf> {
    PopupSelector {
        options,
        display: |path| path.display().to_string(),
        command: |popup| {
            if let Some(path) = popup.options.get(popup.state) {
                return PopupMessage::Open((path.clone(), 0));
            }
            PopupMessage::Done
        },
        size: None,
        state: 0,
    }
}
