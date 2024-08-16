use super::PopupInterface;
use crate::{
    global_state::{Clipboard, GlobalState, PopupMessage, TreeEvent},
    render::{
        backend::{color, Style},
        layout::{LineBuilder, BORDERS},
        state::State,
        TextField,
    },
    tree::Tree,
};
use crossterm::event::{KeyCode, KeyEvent};
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
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        if let Some(msg) = self.pattern.map(key, clipboard) {
            return msg;
        }
        self.updated = true;
        match key.code {
            KeyCode::Up => self.state.prev(self.options.len()),
            KeyCode::Down => self.state.next(self.options.len()),
            KeyCode::Tab => return PopupMessage::Tree(TreeEvent::SearchFiles(self.pattern.text.to_owned())),
            KeyCode::Enter => {
                if self.options.len() > self.state.selected {
                    return TreeEvent::Open(self.options.remove(self.state.selected)).into();
                }
                return PopupMessage::Clear;
            }
            _ => {}
        }
        PopupMessage::None
    }

    fn render(&mut self, gs: &mut GlobalState) {
        let mut area = gs.screen_rect.center(20, 120);
        area.bordered();
        area.draw_borders(None, None, &mut gs.writer);
        area.border_title_styled(PATH_SEARCH_TITLE, Style::fg(color::blue()), &mut gs.writer);
        let mut lines = area.into_iter();
        if let Some(line) = lines.next() {
            self.pattern.widget(line, &mut gs.writer);
        }
        if let Some(line) = lines.next() {
            line.fill(BORDERS.horizontal, &mut gs.writer);
        }
        if let Some(list_rect) = lines.into_rect() {
            if self.options.is_empty() {
                self.state.render_list(["No results found!"].into_iter(), &list_rect, &mut gs.writer);
            } else {
                self.state.render_list_complex(
                    &self.options,
                    &[|path, mut builder| {
                        builder.push(&format!("{}", path.display()));
                    }],
                    &list_rect,
                    &mut gs.writer,
                );
            };
        };
    }

    fn update_tree(&mut self, file_tree: &mut Tree) {
        if self.pattern.text.is_empty() {
            self.options.clear();
        } else {
            self.options = file_tree.search_paths(&self.pattern.text);
        };
        self.updated = true;
        self.state.select(0, self.options.len());
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }

    fn mark_as_updated(&mut self) {}
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
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        if let Some(msg) = self.pattern.map(key, clipboard) {
            return msg;
        }
        self.updated = true;
        match key.code {
            KeyCode::Up => self.state.prev(self.options.len()),
            KeyCode::Down => self.state.next(self.options.len()),
            KeyCode::Tab => {
                if matches!(self.mode, Mode::Full) {
                    return PopupMessage::Clear;
                }
                self.mode = Mode::Full;
                return PopupMessage::Tree(TreeEvent::PopupAccess);
            }
            KeyCode::Enter => {
                if self.options.len() > self.state.selected {
                    let (path, _, line) = self.options.remove(self.state.selected);
                    return TreeEvent::OpenAtLine(path, line).into();
                }
                return PopupMessage::Clear;
            }
            _ => {}
        }
        PopupMessage::None
    }

    fn render(&mut self, gs: &mut GlobalState) {
        let mut area = gs.screen_rect.center(20, 120);
        area.bordered();
        area.draw_borders(None, None, &mut gs.writer);
        match self.mode {
            Mode::Full => area.border_title_styled(FULL_SEARCH_TITLE, Style::fg(color::red()), &mut gs.writer),
            Mode::Select => area.border_title_styled(FILE_SEARCH_TITLE, Style::fg(color::yellow()), &mut gs.writer),
        }
        let mut lines = area.into_iter();
        if let Some(line) = lines.next() {
            self.pattern.widget(line, &mut gs.writer);
        }
        if let Some(line) = lines.next() {
            line.fill(BORDERS.horizontal, &mut gs.writer);
        }
        if let Some(list_rect) = lines.into_rect() {
            if self.options.is_empty() {
                self.state.render_list(["No results found!"].into_iter(), &list_rect, &mut gs.writer);
            } else {
                self.state.render_list_complex(
                    &self.options,
                    &[build_path_line, build_text_line],
                    &list_rect,
                    &mut gs.writer,
                );
            }
        };
    }

    fn fast_render(&mut self, gs: &mut GlobalState) {
        if let Ok(mut buffer) = self.option_buffer.try_lock() {
            if !buffer.is_empty() {
                self.options.extend(buffer.drain(..));
                self.updated = true;
            }
        }
        if self.collect_update_status() {
            self.render(gs);
        }
    }

    fn update_tree(&mut self, file_tree: &mut Tree) {
        self.updated = true;
        if self.pattern.text.len() < 2 {
            self.options.clear();
            return;
        };
        self.options.clear();
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

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }

    fn mark_as_updated(&mut self) {}
}

fn build_path_line((path, ..): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{}", path.display()));
}

fn build_text_line((.., line_txt, line_idx): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{line_idx}| "));
    builder.push(line_txt);
}
