use super::{Button, Popup};
use crate::configs::PopupMessage;
use crossterm::event::KeyCode;

pub fn save_all_popup() -> Popup {
    Popup {
        message: "Not all opened editors are saved!".into(),
        state: 0,
        message_as_buffer_builder: None,
        buttons: vec![
            Button {
                command: |_| PopupMessage::SaveAndExit,
                name: "Save All (Y)".into(),
                key: Some(vec![KeyCode::Char('y'), KeyCode::Char('Y')]),
            },
            Button {
                command: |_| PopupMessage::Exit,
                name: "Don't save (N)".into(),
                key: Some(vec![KeyCode::Char('n'), KeyCode::Char('N')]),
            },
        ],
        size: Some((40, 20)),
    }
}

pub fn go_to_line_popup() -> Popup {
    Popup {
        message: String::new(),
        message_as_buffer_builder: Some(|ch| if ch.is_numeric() { Some(ch) } else { None }),
        buttons: vec![Button {
            command: |popup| {
                if let Ok(line) = popup.message.parse::<usize>() {
                    return PopupMessage::GoToLine(line.checked_sub(1).unwrap_or_default());
                }
                PopupMessage::Done
            },
            name: "GO".into(),
            key: None,
        }],
        size: Some((20, 16)),
        state: 0,
    }
}
