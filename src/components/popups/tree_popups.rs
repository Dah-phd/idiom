use super::{Button, Popup};
use crate::configs::PopupMessage;

pub fn create_file_popup(path: String) -> Popup {
    Popup {
        message: String::new(),
        message_as_buffer_builder: Some(Some),
        buttons: vec![Button {
            command: |popup| PopupMessage::CreatFile(popup.message.to_owned()),
            name: format!("Create in {path}"),
            key: None,
        }],
        size: Some((20, 16)),
        state: 0,
    }
}
