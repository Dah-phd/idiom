use crossterm::event::KeyCode;

use crate::messages::PopupMessage;

use super::{Button, Popup};

pub fn save_all_popup() -> Popup {
    Popup {
        message: "Not all editors are saved! Press Y/Return to save all N/Esc to exit anyway.".into(),
        buttons: vec![
            Button {
                command: || PopupMessage::SaveAndExit,
                name: "Save All (Y)".into(),
                key: Some(vec![KeyCode::Char('y')]),
            },
            Button {
                command: || PopupMessage::Exit,
                name: "Exit (N)".into(),
                key: Some(vec![KeyCode::Char('n')]),
            },
        ],
        size: None,
    }
}
