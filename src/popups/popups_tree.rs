use super::{Popup, PopupSelector};
use crate::{
    global_state::{IdiomEvent, PopupMessage},
    render::Button,
};
use lsp_types::{Location, Range};
use std::path::PathBuf;

pub fn create_file_popup(path: String) -> Box<Popup> {
    let mut buttons = vec![Button {
        command: |popup| IdiomEvent::CreateFileOrFolder(popup.message.to_owned()).into(),
        name: "Create",
        key: None,
    }];
    if path != "./" {
        buttons.push(Button {
            command: |popup| IdiomEvent::CreateFileOrFolderBase(popup.message.to_owned()).into(),
            name: "Create in ./",
            key: None,
        })
    }
    Box::new(Popup::new(String::new(), Some(format!("New in {}", path)), Some(Some), buttons, Some((4, 40))))
}

pub fn rename_file_popup(path: String) -> Box<Popup> {
    Box::new(Popup::new(
        String::new(),
        Some(format!("Rename: {path}")),
        Some(Some),
        vec![Button {
            command: |popup| IdiomEvent::RenameFile(popup.message.to_owned()).into(),
            name: "Rename",
            key: None,
        }],
        Some((4, 40)),
    ))
}

pub fn refrence_selector(options: Vec<Location>) -> Box<PopupSelector<(String, PathBuf, Range)>> {
    Box::new(PopupSelector::new(
        options.into_iter().map(location_with_display).collect(),
        |(display, ..)| display,
        |popup| {
            if let Some((_, path, range)) = popup.options.get(popup.state.selected) {
                return IdiomEvent::OpenAtSelect(path.clone(), (range.start.into(), range.end.into())).into();
            }
            PopupMessage::Clear
        },
        None,
    ))
}

fn location_with_display(loc: Location) -> (String, PathBuf, Range) {
    let path = PathBuf::from(loc.uri.path().as_str());
    let range = loc.range;
    (format!("{} ({})", path.display(), range.start.line + 1), path, range)
}
