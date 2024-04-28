use super::{Popup, PopupSelector};
use crate::{
    global_state::{PopupMessage, TreeEvent},
    render::Button,
};
use lsp_types::{Location, Range};
use std::path::PathBuf;

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

pub fn refrence_selector(mut options: Vec<Location>) -> Box<PopupSelector<(PathBuf, Range)>> {
    Box::new(PopupSelector {
        options: options.drain(..).map(|loc| (PathBuf::from(loc.uri.path()), loc.range)).collect(),
        display: |(path, range)| format!("{} ({})", path.display(), range.start.line + 1),
        command: |popup| {
            if let Some((path, range)) = popup.options.get(popup.state) {
                return TreeEvent::OpenAtSelect(path.clone(), (range.start.into(), range.end.into())).into();
            }
            PopupMessage::Clear
        },
        size: None,
        state: 0,
    })
}
