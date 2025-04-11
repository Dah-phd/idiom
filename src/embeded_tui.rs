use std::time::Duration;

use crate::{
    error::IdiomResult,
    global_state::GlobalState,
    render::{
        backend::{Backend, BackendProtocol},
        pty::PtyShell,
    },
};
const MIN_FRAMERATE: Duration = Duration::from_millis(8);

pub fn run_embeded_tui(cmd: &str, gs: &mut GlobalState) -> IdiomResult<()> {
    let backend = gs.backend();
    let rect = Backend::screen()?;
    let mut tui = PtyShell::run(cmd, rect)?;
    tui.render(backend);
    while !tui.is_finished() {
        if crossterm::event::poll(MIN_FRAMERATE)? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                _ = tui.key_map(&key);
            }
            backend.freeze();
            tui.render(backend);
            backend.unfreeze();
        } else {
            backend.freeze();
            tui.fast_render(backend);
            backend.unfreeze();
        }
    }
    Ok(())
}
