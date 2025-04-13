pub mod generic_popup;
pub mod generic_selector;
pub mod menu;
pub mod pallet;
pub mod popup_file_open;
pub mod popup_find;
pub mod popup_replace;
pub mod popup_tree_search;
pub mod popups_editor;
pub mod popups_tree;
mod utils;
use std::time::Duration;

use crate::{
    app::{MIN_FRAMERATE, MIN_HEIGHT, MIN_WIDTH},
    configs::CONFIG_FOLDER,
    embeded_term::EditorTerminal,
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::{
        backend::{Backend, BackendProtocol, StyleExt},
        layout::Rect,
    },
    tree::Tree,
    workspace::Workspace,
};
use crossterm::{
    event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    style::{Color, ContentStyle},
};
use dirs::config_dir;
use fuzzy_matcher::skim::SkimMatcherV2;
pub use generic_popup::{save_and_exit_popup, Popup};

pub trait PopupInterface {
    fn fast_render(&mut self, screen: Rect, backend: &mut Backend) {
        if self.collect_update_status() {
            self.render(screen, backend);
        }
    }

    fn mouse_map(&mut self, _event: MouseEvent) -> PopupMessage {
        PopupMessage::None
    }

    fn map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, matcher: &SkimMatcherV2) -> PopupMessage {
        self.mark_as_updated();
        match key {
            KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => PopupMessage::Clear,
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => PopupMessage::Clear,
            KeyEvent { code: KeyCode::Esc, .. } => PopupMessage::Clear,
            _ => self.key_map(key, clipboard, matcher),
        }
    }

    fn render(&mut self, screen: Rect, backend: &mut Backend);
    fn resize(&mut self, new_screen: Rect) -> PopupMessage;
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, matcher: &SkimMatcherV2) -> PopupMessage;
    fn component_access(&mut self, _gs: &mut GlobalState, _ws: &mut Workspace, _tree: &mut Tree) {}
    fn mark_as_updated(&mut self);
    fn collect_update_status(&mut self) -> bool;
    fn paste_passthrough(&mut self, _clip: String, _matcher: &SkimMatcherV2) -> PopupMessage {
        PopupMessage::None
    }
}

pub enum Status<T> {
    Result(T),
    Dropped,
    Pending,
}

pub struct Components<'a> {
    pub gs: &'a mut GlobalState,
    pub ws: &'a mut Workspace,
    pub tree: &'a mut Tree,
    pub term: &'a mut EditorTerminal,
}

impl Components<'_> {
    pub fn re_draw(&mut self) {
        self.gs.draw(self.ws, self.tree, self.term);
        self.gs.force_screen_rebuild();
    }
}

pub trait InplacePopup {
    type R;

    fn run(
        &mut self,
        gs: &mut GlobalState,
        ws: &mut Workspace,
        tree: &mut Tree,
        term: &mut EditorTerminal,
    ) -> Option<Self::R> {
        // executed when finish
        let mut components = Components { gs, ws, tree, term };
        components.re_draw();
        self.force_render(components.gs);
        loop {
            if crossterm::event::poll(MIN_FRAMERATE).ok()? {
                match crossterm::event::read().ok()? {
                    Event::Key(key) => match self.map_key(key, &mut components) {
                        Status::Result(value) => return Some(value),
                        Status::Dropped => return None,
                        Status::Pending => (),
                    },
                    Event::Mouse(event) => match self.map_mouse(event, &mut components) {
                        Status::Result(value) => return Some(value),
                        Status::Dropped => return None,
                        Status::Pending => (),
                    },
                    Event::Resize(width, height) => {
                        components.gs.full_resize(height, width);
                        if !self.resize_success(components.gs) {
                            return None;
                        };
                        components.re_draw();
                        self.force_render(components.gs);
                        // executed when finish
                        components.gs.force_screen_rebuild();
                    }
                    Event::Paste(clip) => {
                        if self.paste_passthrough(clip, &mut components) {
                            self.force_render(components.gs);
                        };
                    }
                    _ => (),
                };
            }
            self.render(components.gs);
            components.gs.backend.flush_buf();
        }
    }

    fn paste_passthrough(&mut self, _clip: String, _components: &mut Components) -> bool {
        false
    }

    fn map_key(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R> {
        match key {
            KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => Status::Dropped,
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => Status::Dropped,
            KeyEvent { code: KeyCode::Esc, .. } => Status::Dropped,
            _ => self.map_keyboard(key, components),
        }
    }

    fn render(&mut self, gs: &mut GlobalState);
    fn force_render(&mut self, gs: &mut GlobalState);
    fn resize_success(&mut self, gs: &mut GlobalState) -> bool;
    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R>;
    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status<Self::R>;
}

struct Command {
    label: &'static str,
    result: CommandResult,
}

impl Command {
    fn execute(self) -> CommandResult {
        self.result
    }

    fn cfg_open(label: &'static str, file_path: &'static str) -> Option<Self> {
        let mut path = config_dir()?;
        path.push(CONFIG_FOLDER);
        path.push(file_path);
        Some(Command { label, result: CommandResult::Simple(IdiomEvent::OpenAtLine(path, 0)) })
    }

    fn pass_event(label: &'static str, event: IdiomEvent) -> Self {
        Command { label, result: CommandResult::Simple(event) }
    }

    const fn access_edit(label: &'static str, cb: fn(&mut Workspace, &mut Tree)) -> Self {
        Command { label, result: CommandResult::Complex(cb) }
    }

    fn big_cb(label: &'static str, cb: fn(&mut GlobalState, &mut Workspace, &mut Tree, &mut EditorTerminal)) -> Self {
        Command { label, result: CommandResult::BigCB(cb) }
    }
}

#[derive(Debug, Clone)]
enum CommandResult {
    Simple(IdiomEvent),
    Complex(fn(&mut Workspace, &mut Tree)),
    BigCB(fn(&mut GlobalState, &mut Workspace, &mut Tree, &mut EditorTerminal)),
}

pub fn get_new_screen_size(backend: &mut Backend) -> Option<(u16, u16)> {
    loop {
        if crossterm::event::poll(Duration::from_millis(200)).ok()? {
            match crossterm::event::read().ok()? {
                Event::Key(KeyEvent { code: KeyCode::Char('q' | 'Q' | 'd' | 'D'), .. }) => {
                    return None;
                }
                Event::Resize(width, height) if width >= MIN_WIDTH && height >= MIN_HEIGHT => {
                    return Some((width, height));
                }
                Event::Resize(..) => {}
                _ => continue,
            }
        }
        let error_text = ["Terminal size too small!", "Press Q or D to exit ..."];
        let style = ContentStyle::bold().with_fg(Color::DarkRed);
        let screen = Backend::screen().ok()?;
        let mut text_iter = error_text.iter();
        for line in screen.into_iter() {
            match text_iter.next() {
                Some(text) => line.render_centered_styled(text, style, backend),
                None => line.render_empty(backend),
            }
        }
        backend.flush_buf();
    }
}

pub fn get_init_screen(backend: &mut Backend) -> Option<Rect> {
    let init = Backend::screen().ok()?;
    if init.width < MIN_WIDTH as usize || init.height < MIN_HEIGHT {
        get_new_screen_size(backend)?;
    } else {
        return Some(init);
    };
    Backend::screen().ok()
}
