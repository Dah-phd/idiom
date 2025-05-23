use super::generic_popup::{CommandButton, PopupChoice};
use super::generic_selector::PopupSelector;
use super::Components;
use crate::global_state::IdiomEvent;
use crossterm::event::KeyCode;
use std::path::PathBuf;

pub fn selector_editors(options: Vec<String>) -> PopupSelector<String> {
    PopupSelector::new(
        options,
        |editor, line, backend| line.render(editor, backend),
        |popup, components| {
            let Components { gs, ws, .. } = components;
            ws.activate_editor(popup.state.selected, gs);
            if ws.get_active().is_some() {
                gs.insert_mode();
            }
        },
        None,
    )
}

pub fn file_updated(path: PathBuf) -> PopupChoice {
    PopupChoice::new(
        "File updated! (Use cancel/close to do nothing)".into(),
        None,
        Some(path.display().to_string()),
        None,
        vec![
            CommandButton {
                command: |_, c| c.gs.event.push(IdiomEvent::Save),
                name: "Overwrite (S)",
                key: Some(vec![KeyCode::Char('s'), KeyCode::Char('S')]),
            },
            CommandButton {
                command: |_, c| c.gs.event.push(IdiomEvent::Rebase),
                name: "Rebase (L)",
                key: Some(vec![KeyCode::Char('l'), KeyCode::Char('L')]),
            },
        ],
        Some((4, 60)),
    )
}
