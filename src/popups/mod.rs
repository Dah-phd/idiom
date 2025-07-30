pub mod generic_popup;
pub mod generic_selector;
pub mod menu;
pub mod pallet;
pub mod popup_file_open;
pub mod popup_find;
pub mod popup_lsp_select;
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
    error::{IdiomError, IdiomResult},
    ext_tui::{CrossTerm, StyleExt},
    global_state::{GlobalState, IdiomEvent},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::{
    event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    style::{Color, ContentStyle},
};
use dirs::config_dir;
pub use generic_popup::{should_save_and_exit, PopupChoice};
use idiom_tui::{layout::Rect, Backend};

pub enum Status {
    Finished,
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

    pub fn event(&mut self, event: IdiomEvent) {
        self.gs.event.push(event);
    }
}

pub trait Popup {
    fn run(
        &mut self,
        gs: &mut GlobalState,
        ws: &mut Workspace,
        tree: &mut Tree,
        term: &mut EditorTerminal,
    ) -> IdiomResult<()> {
        // executed when finish
        let mut components = Components { gs, ws, tree, term };
        components.re_draw();
        self.force_render(components.gs);
        components.gs.backend.flush_buf();
        loop {
            if crossterm::event::poll(MIN_FRAMERATE)? {
                match crossterm::event::read()? {
                    Event::Key(key) => {
                        if let Status::Finished = self.map_key(key, &mut components) {
                            return Ok(());
                        }
                    }
                    Event::Mouse(event) => {
                        if let Status::Finished = self.map_mouse(event, &mut components) {
                            return Ok(());
                        }
                    }
                    Event::Resize(width, height) => {
                        let (width, height) = checked_new_screen_size(width, height, components.gs.backend());
                        components.gs.full_resize(height, width);
                        if !self.resize_success(components.gs) {
                            return Ok(());
                        };
                        components.re_draw();
                        self.force_render(components.gs);
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

    fn map_key(&mut self, key: KeyEvent, components: &mut Components) -> Status {
        match key {
            KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => Status::Finished,
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => Status::Finished,
            KeyEvent { code: KeyCode::Esc, .. } => Status::Finished,
            _ => self.map_keyboard(key, components),
        }
    }

    fn render(&mut self, gs: &mut GlobalState);
    fn force_render(&mut self, gs: &mut GlobalState);
    fn resize_success(&mut self, gs: &mut GlobalState) -> bool;
    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status;
    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status;
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

    fn components(
        label: &'static str,
        cb: fn(&mut GlobalState, &mut Workspace, &mut Tree, &mut EditorTerminal),
    ) -> Self {
        Command { label, result: CommandResult::BigCB(cb) }
    }
}

#[derive(Debug, Clone)]
enum CommandResult {
    Simple(IdiomEvent),
    BigCB(fn(&mut GlobalState, &mut Workspace, &mut Tree, &mut EditorTerminal)),
}

type Width = u16;
type Height = u16;

pub fn get_new_screen_size(backend: &mut CrossTerm) -> IdiomResult<(Width, Height)> {
    loop {
        if crossterm::event::poll(Duration::from_millis(200))? {
            match crossterm::event::read()? {
                Event::Key(KeyEvent { code: KeyCode::Char('q' | 'Q' | 'd' | 'D'), .. }) => {
                    return Err(IdiomError::GeneralError(String::from("Canceled terminal resize!")));
                }
                Event::Resize(width, height) if width >= MIN_WIDTH && height >= MIN_HEIGHT => {
                    return Ok((width, height));
                }
                Event::Resize(..) => {}
                _ => continue,
            }
        }
        let error_text = ["Terminal size too small!", "Press Q or D to exit ..."];
        let style = ContentStyle::bold().with_fg(Color::DarkRed);
        let screen = CrossTerm::screen()?;
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

pub fn get_init_screen(backend: &mut CrossTerm) -> IdiomResult<Rect> {
    let init = CrossTerm::screen()?;
    if init.width < MIN_WIDTH as usize || init.height < MIN_HEIGHT {
        get_new_screen_size(backend).map(Rect::from)
    } else {
        Ok(init)
    }
}

pub fn checked_new_screen_size(width: Width, height: Height, backend: &mut CrossTerm) -> (Width, Height) {
    if width >= MIN_WIDTH && height >= MIN_HEIGHT {
        return (width, height);
    }
    get_new_screen_size(backend).expect("Manual action")
}
