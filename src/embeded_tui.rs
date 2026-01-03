use crate::{
    embeded_term::EditorTerminal,
    error::{IdiomError, IdiomResult},
    ext_tui::{
        pty::{Message, PtyShell, OVERLAY_INFO},
        CrossTerm,
    },
    global_state::GlobalState,
    popups::checked_new_screen_size,
    workspace::Workspace,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use idiom_tui::Backend;
use std::time::Duration;
const MIN_FRAMERATE: Duration = Duration::from_millis(8);

pub fn run_embeded_tui(
    cmd: Option<&str>,
    ws: &mut Workspace,
    term: &mut EditorTerminal,
    gs: &mut GlobalState,
) -> IdiomResult<()> {
    let mut rect = CrossTerm::screen()?;
    rect.height -= 1;

    let mut tui = match cmd {
        Some(cmd) => PtyShell::run(cmd, rect)?,
        None => PtyShell::default_cmd(rect)?,
    };

    gs.message(OVERLAY_INFO);
    gs.message(OVERLAY_INFO);
    tui.render(gs.backend());

    while !tui.is_finished() {
        if crossterm::event::poll(MIN_FRAMERATE)? {
            match crossterm::event::read()? {
                Event::Key(KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. }) => {
                    return Ok(());
                }
                Event::Key(key) => {
                    tui.map_key(&key, gs.backend())?;
                }
                Event::Mouse(event) => {
                    if let Message::Copied(clip) = tui.map_mouse(event, gs.backend()) {
                        gs.clipboard.push(clip);
                        gs.success("Select from embeded copied!");
                    }
                }
                Event::Resize(width, height) => {
                    let (width, height) = checked_new_screen_size(width, height, gs.backend());
                    gs.full_resize(ws, term, width, height);
                    gs.render_footer_standalone();
                    let mut rect = CrossTerm::screen()?;
                    rect.height -= 1;
                    tui.resize(rect).map_err(IdiomError::GeneralError)?;
                }
                Event::Paste(clip) => {
                    tui.paste(clip)?;
                }
                _ => (),
            }
            gs.backend.freeze();
            render_message_with_saved_cursor(gs);
            tui.render(&mut gs.backend);
            gs.backend.unfreeze();
        } else {
            gs.backend.freeze();
            render_message_with_saved_cursor(gs);
            tui.fast_render(&mut gs.backend);
            gs.backend.unfreeze();
        }
    }

    Ok(())
}

#[inline]
fn render_message_with_saved_cursor(gs: &mut GlobalState) {
    gs.backend.save_cursor();
    gs.render_footer(None);
    gs.backend.restore_cursor();
}
