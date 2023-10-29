use std::path::PathBuf;

use super::{Button, Popup, PopupSelector};
use crate::configs::PopupMessage;

pub fn create_file_popup(path: String) -> Box<Popup> {
    let mut buttons = vec![Button {
        command: |popup| PopupMessage::CreateFileOrFolder(popup.message.to_owned()),
        name: "Create",
        key: None,
    }];
    if path != "./" {
        buttons.push(Button {
            command: |popup| PopupMessage::CreateFileOrFolderBase(popup.message.to_owned()),
            name: "Create in ./",
            key: None,
        })
    }
    Box::new(Popup {
        message: String::new(),
        message_as_buffer_builder: Some(Some),
        title: Some(format!("New in {}", path)),
        buttons,
        size: Some((40, 4)),
        state: 0,
    })
}

pub fn rename_file_popup(path: String) -> Box<Popup> {
    Box::new(Popup {
        message: String::new(),
        message_as_buffer_builder: Some(Some),
        title: Some(format!("Rename: {path}")),
        buttons: vec![Button {
            command: |popup| PopupMessage::RenameFile(popup.message.to_owned()),
            name: "Rename",
            key: None,
        }],
        size: Some((40, 4)),
        state: 0,
    })
}

pub fn find_in_tree_popup() -> Box<Popup> {
    Box::new(Popup {
        message: String::new(),
        message_as_buffer_builder: Some(Some),
        title: Some("Find in tree".to_owned()),
        buttons: vec![
            Button { command: |popup| PopupMessage::SelectPath(popup.message.to_owned()), name: "Paths", key: None },
            Button {
                command: |popup| PopupMessage::SelectTreeFiles(popup.message.to_owned()),
                name: "Files",
                key: None,
            },
            Button {
                command: |popup| PopupMessage::SelectPathFull(popup.message.to_owned()),
                name: "All paths",
                key: None,
            },
            Button {
                command: |popup| PopupMessage::SelectTreeFilesFull(popup.message.to_owned()),
                name: "All files",
                key: None,
            },
        ],
        size: Some((65, 4)),
        state: 0,
    })
}

pub fn tree_file_selector(options: Vec<(PathBuf, String, usize)>) -> Box<PopupSelector<(PathBuf, String, usize)>> {
    Box::new(PopupSelector {
        options,
        display: |(path, text, idx)| format!("{}\n    {}| {text}", path.display(), idx + 1),
        command: |popup| {
            if let Some((path, _, idx)) = popup.options.get(popup.state) {
                return PopupMessage::Open(path.clone(), *idx);
            }
            PopupMessage::Done
        },
        size: None,
        state: 0,
    })
}

pub fn file_selector(options: Vec<PathBuf>) -> Box<PopupSelector<PathBuf>> {
    Box::new(PopupSelector {
        options,
        display: |path| path.display().to_string(),
        command: |popup| {
            if let Some(path) = popup.options.get(popup.state) {
                return PopupMessage::Open(path.clone(), 0);
            }
            PopupMessage::Done
        },
        size: None,
        state: 0,
    })
}
