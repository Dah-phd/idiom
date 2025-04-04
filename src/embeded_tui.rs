use std::time::Duration;

use crate::{
    global_state::GlobalState,
    render::{
        backend::{Backend, BackendProtocol},
        pty::PtyShell,
    },
};
const MIN_FRAMERATE: Duration = Duration::from_millis(8);

pub fn run_embeded_tui(cmd: &str, gs: &mut GlobalState) {
    let backend = gs.backend();
    let mut rect = Backend::screen().unwrap();
    if rect.height < 6 {
        return;
    }
    rect.height -= 5;
    let mut tui = PtyShell::run(cmd, rect).unwrap();
    tui.render(backend);
    while !tui.is_finished() {
        if crossterm::event::poll(MIN_FRAMERATE).unwrap() {
            match crossterm::event::read().unwrap() {
                crossterm::event::Event::Key(key) => {
                    _ = tui.key_map(&key);
                }
                _ => {}
            }
            tui.render(backend);
        } else {
            tui.fast_render(backend);
        }
    }
}
