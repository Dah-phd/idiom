use crate::{
    embeded_term::EditorTerminal,
    global_state::GlobalState,
    tree::Tree,
    workspace::{cursor::PositionedWord, Workspace},
};
use crossterm::{
    self,
    event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    style::{Color, ContentStyle},
};
use std::time::Duration;

const FRAME_RATE: Duration = Duration::from_millis(250);
const UNFOCUSSED_FRAME_RAGE: Duration = Duration::from_secs(5);

/// Not a real popup
/// uses similar structure to show marked word
/// but does not implement all APIs
pub fn render_marked_word(
    gs: &mut GlobalState,
    ws: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> crate::error::IdiomResult<()> {
    let Some(editor) = ws.get_active() else { return Ok(()) };
    if !editor.file_type.is_code() {
        return Ok(());
    };
    let position = editor.controls.get_base_cursor_position().unwrap_or(editor.cursor.get_position());
    let Some(word) = PositionedWord::find_at(&editor.content, position) else {
        return Ok(());
    };
    let screen_text = editor.content.iter().enumerate().skip(editor.cursor.at_line).take(editor.cursor.max_rows);
    let mut base_words = word.iter_word_ranges(screen_text).collect::<Vec<_>>();
    panic!("{:?}", base_words);
    let mut frame_rate = Duration::from_millis(250);
    loop {
        if crossterm::event::poll(frame_rate)? {
            match crossterm::event::read()? {
                Event::Key(event) => {
                    _ = gs.map_key(&event, ws, tree, term);
                    return Ok(());
                }
                Event::Mouse(event) => {
                    gs.map_mouse(event, tree, ws, term);
                    return Ok(());
                }
                Event::Paste(clip) => {
                    gs.passthrough_paste(clip, ws, term);
                    return Ok(());
                }
                Event::Resize(width, height) => {
                    gs.full_resize(ws, term, width, height);
                    return Ok(());
                }
                Event::FocusGained => {
                    frame_rate = FRAME_RATE;
                }
                Event::FocusLost => {
                    frame_rate = UNFOCUSSED_FRAME_RAGE;
                }
            }
        }
    }
}
