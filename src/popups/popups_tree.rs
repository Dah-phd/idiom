use super::{Popup, PopupSelector};
use crate::{
    global_state::{PopupMessage, TreeEvent},
    render::{state::State, Button},
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
        size: Some((4, 40)),
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
        size: Some((4, 40)),
        state: 0,
    })
}

pub fn refrence_selector(options: Vec<Location>) -> Box<PopupSelector<(String, PathBuf, Range)>> {
    Box::new(PopupSelector {
        options: options.into_iter().map(location_with_display).collect(),
        display: |(display, ..)| &display,
        command: |popup| {
            if let Some((_, path, range)) = popup.options.get(popup.state.selected) {
                return TreeEvent::OpenAtSelect(path.clone(), (range.start.into(), range.end.into())).into();
            }
            PopupMessage::Clear
        },
        size: None,
        state: State::new(),
    })
}

fn location_with_display(loc: Location) -> (String, PathBuf, Range) {
    let path = PathBuf::from(loc.uri.path());
    let range = loc.range;
    (format!("{} ({})", path.display(), range.start.line + 1), path, range)
}
