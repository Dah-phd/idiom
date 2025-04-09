use super::PopupInterface;
use crate::{
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::{
        backend::{Backend, StyleExt},
        layout::{IterLines, LineBuilder, Rect, BORDERS},
        state::State,
        TextField,
    },
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::{Color, ContentStyle};
use fuzzy_matcher::skim::SkimMatcherV2;
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

type SearchResult = (PathBuf, String, usize);

const PATH_SEARCH_TITLE: &str = " Path search (Tab to switch to in File search) ";
const FILE_SEARCH_TITLE: &str = " File search (Selected - Tab to switch to Full mode) ";
const FULL_SEARCH_TITLE: &str = " File search (Full) ";

pub struct ActivePathSearch {
    options: Vec<PathBuf>,
    state: State,
    pattern: TextField<PopupMessage>,
    updated: bool,
}

impl ActivePathSearch {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            options: Vec::new(),
            state: State::default(),
            pattern: TextField::with_tree_access(String::new()),
            updated: true,
        })
    }
}

impl PopupInterface for ActivePathSearch {
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
        if let Some(msg) = self.pattern.map(key, clipboard) {
            return msg;
        }
        match key.code {
            KeyCode::Up => self.state.prev(self.options.len()),
            KeyCode::Down => self.state.next(self.options.len()),
            KeyCode::Tab => return PopupMessage::Event(IdiomEvent::SearchFiles(self.pattern.text.to_owned())),
            KeyCode::Enter => {
                if self.options.len() > self.state.selected {
                    return IdiomEvent::OpenAtLine(self.options.remove(self.state.selected), 0).into();
                }
                return PopupMessage::Clear;
            }
            _ => {}
        }
        PopupMessage::None
    }

    fn render(&mut self, screen: Rect, backend: &mut Backend) {
        let mut area = screen.center(20, 120);
        area.bordered();
        area.draw_borders(None, None, backend);
        area.border_title_styled(PATH_SEARCH_TITLE, ContentStyle::fg(Color::Blue), backend);
        let mut lines = area.into_iter();
        if let Some(line) = lines.next() {
            self.pattern.widget(line, backend);
        }
        if let Some(line) = lines.next() {
            line.fill(BORDERS.horizontal_top, backend);
        }
        if let Some(list_rect) = lines.into_rect() {
            if self.options.is_empty() {
                self.state.render_list(["No results found!"].into_iter(), list_rect, backend);
            } else {
                self.state.render_list_complex(
                    &self.options,
                    &[|path, mut builder| {
                        builder.push(&format!("{}", path.display()));
                    }],
                    &list_rect,
                    backend,
                );
            };
        };
    }

    fn resize(&mut self, _new_screen: Rect) -> PopupMessage {
        self.mark_as_updated();
        PopupMessage::None
    }

    fn component_access(&mut self, _gs: &mut GlobalState, _ws: &mut Workspace, tree: &mut Tree) {
        if self.pattern.text.is_empty() {
            self.options.clear();
        } else {
            self.options = tree.search_paths(&self.pattern.text);
        };
        self.mark_as_updated();
        self.state.reset();
    }

    fn paste_passthrough(&mut self, clip: String, _: &SkimMatcherV2) -> PopupMessage {
        self.pattern.paste_passthrough(clip)
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }

    fn mark_as_updated(&mut self) {
        self.updated = true;
    }
}

enum Mode {
    Full,
    Select,
}

pub struct ActiveFileSearch {
    join_handle: Option<JoinHandle<()>>,
    options: Vec<SearchResult>,
    option_buffer: Arc<Mutex<Vec<SearchResult>>>,
    state: State,
    mode: Mode,
    pattern: TextField<PopupMessage>,
    updated: bool,
}

impl ActiveFileSearch {
    pub fn new(pattern: String) -> Box<Self> {
        Box::new(Self {
            mode: Mode::Select,
            join_handle: None,
            option_buffer: Arc::default(),
            options: Vec::default(),
            state: State::default(),
            pattern: TextField::with_tree_access(pattern),
            updated: true,
        })
    }
}

impl PopupInterface for ActiveFileSearch {
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
        if let Some(msg) = self.pattern.map(key, clipboard) {
            return msg;
        }
        match key.code {
            KeyCode::Up => self.state.prev(self.options.len()),
            KeyCode::Down => self.state.next(self.options.len()),
            KeyCode::Tab => {
                if matches!(self.mode, Mode::Full) {
                    return PopupMessage::Clear;
                }
                self.mode = Mode::Full;
                return PopupMessage::Event(IdiomEvent::PopupAccess);
            }
            KeyCode::Enter => {
                if self.options.len() > self.state.selected {
                    let (path, _, line) = self.options.remove(self.state.selected);
                    return IdiomEvent::OpenAtLine(path, line).into();
                }
                return PopupMessage::Clear;
            }
            _ => {}
        }
        PopupMessage::None
    }

    fn render(&mut self, screen: Rect, backend: &mut Backend) {
        let mut area = screen.center(20, 120);
        area.bordered();
        area.draw_borders(None, None, backend);
        match self.mode {
            Mode::Full => area.border_title_styled(FULL_SEARCH_TITLE, ContentStyle::fg(Color::Red), backend),
            Mode::Select => area.border_title_styled(FILE_SEARCH_TITLE, ContentStyle::fg(Color::Yellow), backend),
        }
        let mut lines = area.into_iter();
        if let Some(line) = lines.next() {
            self.pattern.widget(line, backend);
        }
        if let Some(line) = lines.next() {
            line.fill(BORDERS.horizontal_top, backend);
        }
        if let Some(list_rect) = lines.into_rect() {
            if self.options.is_empty() {
                self.state.render_list(["No results found!"].into_iter(), list_rect, backend);
            } else {
                self.state.render_list_complex(&self.options, &[build_path_line, build_text_line], &list_rect, backend);
            }
        };
    }

    fn resize(&mut self, _new_screen: Rect) -> PopupMessage {
        self.mark_as_updated();
        PopupMessage::None
    }

    fn fast_render(&mut self, screen: Rect, backend: &mut Backend) {
        if let Ok(mut buffer) = self.option_buffer.try_lock() {
            if !buffer.is_empty() {
                self.options.extend(buffer.drain(..));
                self.updated = true;
            }
        }
        if self.collect_update_status() {
            self.render(screen, backend);
        }
    }

    fn component_access(&mut self, _gs: &mut GlobalState, _ws: &mut Workspace, file_tree: &mut Tree) {
        self.mark_as_updated();
        if self.pattern.text.len() < 2 {
            self.options.clear();
            return;
        };
        self.options.clear();
        self.state.reset();
        let tree_path = match self.mode {
            Mode::Full => file_tree.shallow_copy_root_tree_path(),
            Mode::Select => file_tree.shallow_copy_selected_tree_path(),
        };
        let buffer = Arc::clone(&self.option_buffer);
        let pattern = self.pattern.text.to_owned();
        if let Some(old_handle) = self.join_handle.replace(tokio::task::spawn(async move {
            buffer.lock().await.clear();
            let mut join_set = tree_path.search_files_join_set(pattern);
            while let Some(task_result) = join_set.join_next().await {
                if let Ok(result) = task_result {
                    buffer.lock().await.extend(result);
                };
            }
        })) {
            if !old_handle.is_finished() {
                old_handle.abort();
            }
        }
    }

    fn paste_passthrough(&mut self, clip: String, _: &SkimMatcherV2) -> PopupMessage {
        self.pattern.paste_passthrough(clip)
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }

    fn mark_as_updated(&mut self) {
        self.updated = true;
    }
}

fn build_path_line((path, ..): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{}", path.display()));
}

fn build_text_line((.., line_txt, line_idx): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{line_idx}| "));
    builder.push(line_txt);
}
