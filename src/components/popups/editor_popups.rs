use super::{Button, Popup};
use crate::messages::PopupMessage;
use crossterm::event::KeyCode;

pub fn save_all_popup() -> Popup {
    Popup {
        message: "Not all opened editors are saved!".into(),
        state: 0,
        buttons: vec![
            Button {
                command: || PopupMessage::SaveAndExit,
                name: "Save All (Y)".into(),
                key: Some(vec![KeyCode::Char('y'), KeyCode::Char('Y')]),
            },
            Button {
                command: || PopupMessage::Exit,
                name: "Don't save (N)".into(),
                key: Some(vec![KeyCode::Char('n'), KeyCode::Char('N')]),
            },
        ],
        size: Some((40, 20)),
    }
}
