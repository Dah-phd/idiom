use super::PopupSelector;
use crate::global_state::{IdiomEvent, PopupMessage};
use crate::workspace::CursorPosition;

pub fn selector_ranges(
    options: Vec<((CursorPosition, CursorPosition), String)>,
) -> Box<PopupSelector<((CursorPosition, CursorPosition), String)>> {
    Box::new(PopupSelector::new(
        options,
        // display: |((from, _), line)| format!("({}) {line}", from.line + 1),
        |((..), line)| line,
        |popup| {
            let (from, to) = popup.options[popup.state.selected].0;
            PopupMessage::ClearEvent(IdiomEvent::GoToSelect { from, to })
        },
        None,
    ))
}

pub fn selector_editors(options: Vec<String>) -> Box<PopupSelector<String>> {
    Box::new(PopupSelector::new(
        options,
        |editor| editor,
        |popup| PopupMessage::ClearEvent(IdiomEvent::ActivateEditor(popup.state.selected)),
        None,
    ))
}
