use super::{Components, Popup, Status};
use crate::{
    embeded_term::EditorTerminal,
    ext_tui::{text_field::TextField, State, StyleExt},
    global_state::{GlobalState, IdiomEvent},
    tree::{Tree, TreePath},
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::Color;
use idiom_tui::layout::{Rect, BORDERS};
use std::{path::PathBuf, sync::Arc, time::Duration};
use std::{sync::Mutex, time::Instant};
use tokio::task::JoinHandle;

const WAIT_ON_UPDATE: Duration = Duration::from_millis(100);

const PATH_SEARCH_TITLE: &str = "Path search (Tab for File search)";

pub struct ActivePathSearch {
    options: Vec<PathBuf>,
    options_buffer: Arc<Mutex<Vec<PathBuf>>>,
    clock: Option<Instant>,
    state: State,
    pattern: TextField<bool>,
    join_handle: Option<JoinHandle<()>>,
    tree: TreePath,
}

impl ActivePathSearch {
    pub fn run(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let mut popup = Self {
            options: Vec::new(),
            options_buffer: Arc::default(),
            clock: None,
            state: State::default(),
            pattern: TextField::new(String::new(), Some(true)),
            join_handle: None,
            tree: tree.shallow_copy_root_tree_path(),
        };
        if let Err(error) = popup.run(gs, ws, tree, term) {
            gs.error(error);
        };
    }

    fn collect_data(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            handle.abort();
        }
        self.options.clear();
        self.state.reset();

        if self.pattern.text.len() < 2 {
            self.clock = None;
        } else {
            self.clock = Some(Instant::now());
        }
    }

    fn get_rect(gs: &GlobalState) -> Rect {
        gs.screen_rect.center(20, 120).with_borders()
    }

    fn get_option_idx(&self, row: u16, column: u16, gs: &GlobalState) -> Option<usize> {
        let mut rect = Self::get_rect(gs);
        rect.height = rect.height.checked_sub(2)?;
        rect.row += 2;
        let position = rect.relative_position(row, column)?;
        let idx = self.state.at_line + position.row as usize;
        if idx >= self.options.len() {
            return None;
        }
        Some(idx)
    }
}

impl Popup for ActivePathSearch {
    fn force_render(&mut self, gs: &mut GlobalState) {
        let mut rect = Self::get_rect(gs);
        let accent_style = gs.theme.accent_style().with_fg(Color::Blue);
        let backend = gs.backend();
        rect.draw_borders(None, None, backend);
        rect.border_title_styled(PATH_SEARCH_TITLE, accent_style, backend);

        let Some(line) = rect.next_line() else { return };
        self.pattern.widget_with_count(line, self.options.len(), backend);
        let Some(line) = rect.next_line() else { return };
        line.fill(BORDERS.horizontal_top, backend);

        if self.clock.is_some() || self.join_handle.is_some() {
            self.state.render_list(["Searching ..."].into_iter(), rect, backend);
            return;
        }

        if self.options.is_empty() {
            self.state.render_list(["No results found!"].into_iter(), rect, backend);
        } else {
            self.state.render_list_complex(
                &self.options,
                &[|path, mut builder| {
                    builder.push(&format!("{}", path.display()));
                }],
                rect,
                backend,
            );
        };
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status {
        let Components { gs, .. } = components;
        if let Some(update) = self.pattern.map(&key, &mut gs.clipboard) {
            if update {
                self.collect_data();
            }
            self.force_render(gs);
            return Status::Pending;
        }
        match key.code {
            KeyCode::Up => self.state.prev(self.options.len()),
            KeyCode::Down => self.state.next(self.options.len()),
            KeyCode::Tab => {
                gs.event.push(IdiomEvent::SearchFiles(self.pattern.text.to_owned()));
                return Status::Finished;
            }
            KeyCode::Enter => {
                if self.options.len() > self.state.selected {
                    gs.event.push(IdiomEvent::OpenAtLine(self.options.remove(self.state.selected), 0));
                }
                return Status::Finished;
            }
            _ => return Status::Pending,
        }
        self.force_render(gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status {
        let Components { gs, .. } = components;
        match event {
            MouseEvent { kind: MouseEventKind::Moved, column, row, .. } => match self.get_option_idx(row, column, gs) {
                Some(idx) => self.state.select(idx, self.options.len()),
                None => return Status::Pending,
            },
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => {
                if let Some(index) = self.get_option_idx(row, column, gs) {
                    gs.event.push(IdiomEvent::OpenAtLine(self.options.remove(index), 0));
                    return Status::Finished;
                }
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => self.state.prev(self.options.len()),
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => self.state.next(self.options.len()),
            _ => return Status::Pending,
        }
        self.force_render(gs);
        Status::Pending
    }

    fn render(&mut self, gs: &mut GlobalState) {
        if matches!(self.clock, Some(inst) if inst.elapsed() >= WAIT_ON_UPDATE) {
            self.clock = None;
            if !self.pattern.text.is_empty() {
                let root_tree = self.tree.clone();
                let pattern = self.pattern.text.to_owned();
                let buffer = Arc::clone(&self.options_buffer);
                self.join_handle.replace(tokio::task::spawn(async move {
                    if let Ok(options) = root_tree.search_tree_paths(&pattern) {
                        let mut lock = match buffer.lock() {
                            Ok(lock) => lock,
                            Err(err) => err.into_inner(),
                        };
                        *lock = options;
                    };
                }));
            };
        } else if matches!(&mut self.join_handle, Some(handle) if handle.is_finished()) {
            let mut lock = match self.options_buffer.lock() {
                Ok(lock) => lock,
                Err(err) => err.into_inner(),
            };
            if !lock.is_empty() {
                self.options = lock.drain(..).collect();
            }
            drop(lock);
            self.join_handle = None;
            self.force_render(gs);
        }
    }

    fn resize_success(&mut self, _: &mut GlobalState) -> bool {
        true
    }

    fn paste_passthrough(&mut self, clip: String, _: &mut Components) -> bool {
        if self.pattern.paste_passthrough(clip) {
            self.collect_data();
            return true;
        }
        false
    }
}
