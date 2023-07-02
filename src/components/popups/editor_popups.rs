use super::{Button, Popup};
use crate::messages::PopupMessage;
use crossterm::event::KeyCode;

pub fn save_all_popup() -> Popup {
    Popup {
        message: "Not all editors are saved! Press Y/Return to save all N/Esc to exit anyway.".into(),
        state: 0,
        buttons: vec![
            Button {
                command: || PopupMessage::SaveAndExit,
                name: "Save All (Y)".into(),
                key: Some(vec![KeyCode::Char('y'), KeyCode::Char('Y')]),
            },
            Button {
                command: || PopupMessage::Exit,
                name: "Exit (N)".into(),
                key: Some(vec![KeyCode::Char('n'), KeyCode::Char('N')]),
            },
        ],
        size: Some((40, 20)),
    }
}
