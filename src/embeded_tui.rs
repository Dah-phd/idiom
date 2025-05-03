use crate::{
    error::{IdiomError, IdiomResult},
    global_state::GlobalState,
    popups::checked_new_screen_size,
    render::{
        backend::{Backend, BackendProtocol},
        pty::PtyShell,
    },
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
const MIN_FRAMERATE: Duration = Duration::from_millis(8);

pub fn run_embeded_tui(cmd: Option<&str>, gs: &mut GlobalState) -> IdiomResult<()> {
    let mut rect = Backend::screen()?;
    rect.height -= 1;

    let mut tui = match cmd {
        Some(cmd) => PtyShell::run(cmd, rect)?,
        None => PtyShell::default_cmd(rect)?,
    };

    tui.render(gs.backend());

    while !tui.is_finished() {
        if crossterm::event::poll(MIN_FRAMERATE)? {
            match crossterm::event::read()? {
                Event::Key(KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. }) => {
                    return Ok(());
                }
                Event::Key(key) => {
                    tui.map_key(&key)?;
                }
                Event::Mouse(event) => tui.map_mouse(event),
                Event::Resize(width, height) => {
                    let (width, height) = checked_new_screen_size(width, height, gs.backend());
                    gs.full_resize(height, width);
                    gs.render_footer_standalone();
                    let mut rect = Backend::screen()?;
                    rect.height -= 1;
                    tui.resize(rect).map_err(IdiomError::GeneralError)?;
                }
                Event::Paste(clip) => {
                    tui.paste(clip)?;
                }
                _ => (),
            }
            gs.backend.freeze();
            gs.fast_render_message_with_preserved_cursor();
            tui.render(&mut gs.backend);
            gs.backend.unfreeze();
        } else {
            gs.backend.freeze();
            gs.fast_render_message_with_preserved_cursor();
            tui.fast_render(&mut gs.backend);
            gs.backend.unfreeze();
        }
    }

    Ok(())
}
