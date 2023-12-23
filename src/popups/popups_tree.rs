use std::path::PathBuf;

use lsp_types::Location;

use super::{Button, Popup, PopupSelector};
use crate::global_state::{PopupMessage, TreeEvent};

pub fn create_file_popup(path: String) -> Box<Popup> {
    let mut buttons = vec![Button {
        command: |popup| TreeEvent::CreateFileOrFolder(popup.message.to_owned()).into(),
        name: "Create",
        key: None,
    }];
    if path != "./" {
        buttons.push(Button {
            command: |popup| TreeEvent::CreateFileOrFolderBase(popup.message.to_owned()).into(),
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
            command: |popup| TreeEvent::RenameFile(popup.message.to_owned()).into(),
            name: "Rename",
            key: None,
        }],
        size: Some((40, 4)),
        state: 0,
    })
}

pub fn search_tree_files(pattern: String) -> Box<Popup> {
    Box::new(Popup {
        message: pattern,
        message_as_buffer_builder: Some(Some),
        title: Some("Find in tree".to_owned()),
        buttons: vec![
            Button {
                command: |popup| TreeEvent::SelectTreeFiles(popup.message.to_owned()).into(),
                name: "Files",
                key: None,
            },
            Button {
                command: |popup| TreeEvent::SelectTreeFilesFull(popup.message.to_owned()).into(),
                name: "All files",
                key: None,
            },
        ],
        size: Some((55, 4)),
        state: 0,
    })
}

pub fn tree_file_selector(options: Vec<(PathBuf, String, usize)>) -> Box<PopupSelector<(PathBuf, String, usize)>> {
    Box::new(PopupSelector {
        options,
        display: |(path, text, idx)| format!("{}\n    {}| {text}", path.display(), idx + 1),
        command: |popup| {
            if let Some((path, _, idx)) = popup.options.get(popup.state) {
                return PopupMessage::Tree(TreeEvent::OpenAtLine(path.clone(), *idx));
            }
            PopupMessage::Clear
        },
        size: None,
        state: 0,
    })
}

pub fn refrence_selector(mut options: Vec<Location>) -> Box<PopupSelector<(PathBuf, usize)>> {
    Box::new(PopupSelector {
        options: options.drain(..).map(|loc| (PathBuf::from(loc.uri.path()), loc.range.start.line as usize)).collect(),
        display: |(path, idx)| format!("{} ({idx})", path.display()),
        command: |popup| {
            if let Some((path, idx)) = popup.options.get(popup.state) {
                return TreeEvent::OpenAtLine(path.clone(), *idx).into();
            }
            PopupMessage::Clear
        },
        size: None,
        state: 0,
    })
}
