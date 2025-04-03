use crossterm::event::KeyEvent;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::{
    error::IdiomResult,
    global_state::{Clipboard, PopupMessage},
    render::{
        backend::{Backend, BackendProtocol},
        layout::Rect,
        pty::PtyShell,
    },
};

use super::PopupInterface;

pub struct EmbededTuiApp {
    shell: PtyShell,
}

impl EmbededTuiApp {
    pub fn new(cmd: &str) -> IdiomResult<Self> {
        let rect = Backend::screen()?;
        PtyShell::run(cmd, rect).map(|shell| Self { shell })
    }
}

impl PopupInterface for EmbededTuiApp {
    fn collect_update_status(&mut self) -> bool {
        true
    }
    fn key_map(&mut self, key: &KeyEvent, _clipboard: &mut Clipboard, _matcher: &SkimMatcherV2) -> PopupMessage {
        let Ok(..) = self.shell.key_map(key) else {
            return PopupMessage::Clear;
        };
        if self.shell.is_finished() {
            return PopupMessage::Clear;
        }
        PopupMessage::None
    }

    fn fast_render(&mut self, _screen: Rect, backend: &mut Backend) {
        self.shell.fast_render(backend);
    }

    fn render(&mut self, screen_rect: Rect, backend: &mut Backend) {
        _ = self.shell.resize(screen_rect);
        self.shell.render(backend);
    }

    fn resize(&mut self, new_rect: Rect) -> PopupMessage {
        if self.shell.resize(new_rect).is_err() {
            return PopupMessage::Clear;
        };
        PopupMessage::None
    }

    fn mark_as_updated(&mut self) {}
}
