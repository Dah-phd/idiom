use crate::{
    global_state::{messages::PopupMessage, WorkspaceEvent},
    workspace::CursorPosition,
};

pub fn into_message(maybe_position: Option<(CursorPosition, CursorPosition)>) -> PopupMessage {
    if let Some(select) = maybe_position {
        WorkspaceEvent::GoToSelect { select, should_clear: false }.into()
    } else {
        PopupMessage::None
    }
}

pub fn next_option<T: Clone>(options: &Vec<T>, state: &mut usize) -> Option<T> {
    if options.len() - 1 > *state {
        *state += 1;
    } else {
        *state = 0;
    }
    options.get(*state).cloned()
}

pub fn prev_option<T: Clone>(options: &Vec<T>, state: &mut usize) -> Option<T> {
    if *state > 0 {
        *state -= 1;
    } else {
        *state = options.len() - 1;
    }
    options.get(*state).cloned()
}

pub fn count_as_string<T>(options: &Vec<T>) -> String {
    let len = options.len();
    if len < 10 {
        format!("  {len}")
    } else if len < 100 {
        format!(" {len}")
    } else {
        String::from("99+")
    }
}
