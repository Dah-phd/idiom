use super::{Popup, PopupSelector};
use crate::{
    global_state::{IdiomEvent, PopupMessage},
    render::Button,
};
use lsp_types::{Location, Range};
use std::path::PathBuf;

pub fn create_file_popup(path: PathBuf) -> Box<Popup> {
    let buttons = vec![
        Button {
            command: |popup| IdiomEvent::CreateFileOrFolder { name: popup.message.to_owned(), from_base: false }.into(),
            name: "Create",
            key: None,
        },
        Button {
            command: |popup| IdiomEvent::CreateFileOrFolder { name: popup.message.to_owned(), from_base: true }.into(),
            name: "Create in ./",
            key: None,
        },
    ];
    Box::new(Popup::new(
        String::new(),
        Some("New in "),
        Some(path.display().to_string()),
        Some(Some),
        buttons,
        Some((4, 40)),
    ))
}

pub fn create_root_file_popup() -> Box<Popup> {
    let buttons = vec![Button {
        command: |popup| IdiomEvent::CreateFileOrFolder { name: popup.message.to_owned(), from_base: true }.into(),
        name: "Create",
        key: None,
    }];
    Box::new(Popup::new(String::new(), Some("New in root dir"), None, Some(Some), buttons, Some((4, 40))))
}

pub fn rename_file_popup(path: String) -> Box<Popup> {
    let message = path.split(std::path::MAIN_SEPARATOR).last().map(ToOwned::to_owned).unwrap_or_default();
    Box::new(Popup::new(
        message,
        Some("Rename: "),
        Some(path),
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
