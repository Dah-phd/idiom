use crate::{configs::EditorAction, global_state::Clipboard};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use idiom_tui::text_field::{Status, TextField as Field};

pub fn map_action(text_field: &mut Field, action: EditorAction, clipbaord: &mut Clipboard) -> Option<Status> {
    match action {
        EditorAction::Copy => {
            if let Some(clip) = text_field.copy() {
                clipbaord.push(clip);
            }
            Some(Status::Skipped)
        }
        EditorAction::Cut => match text_field.cut() {
            Some(clip) => {
                clipbaord.push(clip);
                Some(Status::Updated)
            }
            None => Some(Status::Skipped),
        },
        EditorAction::Paste => match clipbaord.pull() {
            Some(clip) => Some(text_field.paste_passthrough(clip)),
            None => Some(Status::default()),
        },
        EditorAction::Char(ch) => Some(text_field.push_char(ch)),
        EditorAction::Delete => Some(text_field.del()),
        EditorAction::Backspace => Some(text_field.backspace()),
        EditorAction::StartOfLine | EditorAction::StartOfFile => Some(text_field.start_of_line()),
        EditorAction::EndOfLine | EditorAction::EndOfFile => Some(text_field.end_of_line()),
        EditorAction::Left => Some(text_field.go_left()),
        EditorAction::SelectLeft => Some(text_field.select_left()),
        EditorAction::JumpLeft => Some(text_field.jump_left()),
        EditorAction::JumpLeftSelect => Some(text_field.select_jump_left()),
        EditorAction::Right => Some(text_field.go_right()),
        EditorAction::SelectRight => Some(text_field.select_right()),
        EditorAction::JumpRight => Some(text_field.jump_right()),
        EditorAction::JumpRightSelect => Some(text_field.select_jump_right()),
        EditorAction::SelectAll => Some(text_field.select_all()),
        _ => None,
    }
}

pub fn map_key(text_field: &mut Field, key: KeyEvent, clipbaord: &mut Clipboard) -> Option<Status> {
    match key.code {
        KeyCode::Char('c' | 'C')
            if key.modifiers == KeyModifiers::CONTROL
                || key.modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT =>
        {
            if let Some(clip) = text_field.copy() {
                clipbaord.push(clip);
            }
            Some(Status::Skipped)
        }
        KeyCode::Char('x' | 'X') if key.modifiers == KeyModifiers::CONTROL => match text_field.cut() {
            Some(clip) => {
                clipbaord.push(clip);
                Some(Status::Updated)
            }
            None => Some(Status::Skipped),
        },
        KeyCode::Char('v' | 'V') if key.modifiers == KeyModifiers::CONTROL => match clipbaord.pull() {
            Some(clip) => Some(text_field.paste_passthrough(clip)),
            None => Some(Status::default()),
        },
        KeyCode::Char('a' | 'A') if key.modifiers == KeyModifiers::CONTROL => Some(text_field.select_all()),
        KeyCode::Char(ch) => Some(text_field.push_char(ch)),
        KeyCode::Delete => Some(text_field.del()),
        KeyCode::Backspace => Some(text_field.backspace()),
        KeyCode::Home => Some(text_field.start_of_line()),
        KeyCode::End => Some(text_field.end_of_line()),
        KeyCode::Left if key.modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT => {
            Some(text_field.select_jump_left())
        }
        KeyCode::Left if key.modifiers == KeyModifiers::CONTROL => Some(text_field.jump_left()),
        KeyCode::Left if key.modifiers == KeyModifiers::SHIFT => Some(text_field.select_left()),
        KeyCode::Left => Some(text_field.go_left()),
        KeyCode::Right if key.modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT => {
            Some(text_field.select_jump_right())
        }
        KeyCode::Right if key.modifiers == KeyModifiers::CONTROL => Some(text_field.jump_right()),
        KeyCode::Right if key.modifiers == KeyModifiers::SHIFT => Some(text_field.select_right()),
        KeyCode::Right => Some(text_field.go_right()),
        _ => None,
    }
}
