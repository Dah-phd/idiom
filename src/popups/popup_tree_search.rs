use super::{Components, Popup, Status};
use crate::{
    embeded_term::EditorTerminal,
    global_state::{GlobalState, IdiomEvent},
    render::{
        backend::StyleExt,
        layout::{IterLines, LineBuilder, BORDERS},
        state::State,
        TextField,
    },
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::MouseEvent;
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::{Color, ContentStyle};
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

type SearchResult = (PathBuf, String, usize);

const PATH_SEARCH_TITLE: &str = " Path search (Tab to switch to in File search) ";
const FILE_SEARCH_TITLE: &str = " File search (Selected - Tab to switch to Full mode) ";
const FULL_SEARCH_TITLE: &str = " File search (Full) ";

pub struct ActivePathSearch {
    options: Vec<PathBuf>,
    state: State,
    pattern: TextField<bool>,
}

impl ActivePathSearch {
    pub fn run(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        Self { options: Vec::new(), state: State::default(), pattern: TextField::new(String::new(), Some(true)) }
            .run(gs, ws, tree, term);
    }

    fn collect_data(&mut self, tree: &mut Tree) {
        if self.pattern.text.is_empty() {
            self.options.clear();
        } else {
            self.options = tree.search_paths(&self.pattern.text);
        };
        self.state.reset();
    }
}

impl Popup for ActivePathSearch {
    type R = ();

    fn force_render(&mut self, gs: &mut GlobalState) {
        let mut area = gs.screen_rect.center(20, 120);
        let backend = gs.backend();
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

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R> {
        let Components { gs, tree, .. } = components;
        if let Some(update) = self.pattern.map(&key, &mut gs.clipboard) {
            if update {
                self.collect_data(tree);
            }
            self.force_render(gs);
            return Status::Pending;
        }
        match key.code {
            KeyCode::Up => self.state.prev(self.options.len()),
            KeyCode::Down => self.state.next(self.options.len()),
            KeyCode::Tab => {
                gs.event.push(IdiomEvent::SearchFiles(self.pattern.text.to_owned()));
                return Status::Dropped;
            }
            KeyCode::Enter => {
                if self.options.len() > self.state.selected {
                    gs.event.push(IdiomEvent::OpenAtLine(self.options.remove(self.state.selected), 0));
                }
                return Status::Dropped;
            }
            _ => return Status::Pending,
        }
        self.force_render(gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status<Self::R> {
        todo!()
    }

    fn render(&mut self, _: &mut GlobalState) {}

    fn resize_success(&mut self, _: &mut GlobalState) -> bool {
        true
    }

    fn paste_passthrough(&mut self, clip: String, _: &mut Components) -> bool {
        self.pattern.paste_passthrough(clip)
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
    pattern: TextField<bool>,
}

impl ActiveFileSearch {
    pub fn run(pattern: String, gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let mut new = Self {
            mode: Mode::Select,
            join_handle: None,
            option_buffer: Arc::default(),
            options: Vec::default(),
            state: State::default(),
            pattern: TextField::new(pattern, Some(true)),
        };

        if new.pattern.text.len() > 1 {
            new.collect_data(tree);
        }

        new.run(gs, ws, tree, term);
    }

    fn collect_data(&mut self, file_tree: &mut Tree) {
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
        if !self.options.is_empty() {
            panic!("{:?}", self.options.len())
        }
    }
}

impl Popup for ActiveFileSearch {
    type R = ();

    fn force_render(&mut self, gs: &mut GlobalState) {
        let mut area = gs.screen_rect.center(20, 120);
        let backend = gs.backend();
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

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R> {
        let Components { gs, tree, .. } = components;

        if let Some(updated) = self.pattern.map(&key, &mut gs.clipboard) {
            if updated {
                self.collect_data(tree);
            }
            self.force_render(gs);
            return Status::Pending;
        }
        match key.code {
            KeyCode::Up => self.state.prev(self.options.len()),
            KeyCode::Down => self.state.next(self.options.len()),
            KeyCode::Tab => {
                if matches!(self.mode, Mode::Full) {
                    return Status::Dropped;
                }
                self.mode = Mode::Full;
                self.collect_data(tree);
            }
            KeyCode::Enter => {
                if self.options.len() > self.state.selected {
                    let (path, _, line) = self.options.remove(self.state.selected);
                    gs.event.push(IdiomEvent::OpenAtLine(path, line));
                }
                return Status::Dropped;
            }
            _ => return Status::Pending,
        }
        self.force_render(gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status<Self::R> {
        todo!()
    }

    fn render(&mut self, gs: &mut GlobalState) {
        {
            let Ok(mut buffer) = self.option_buffer.try_lock() else {
                return;
            };
            if buffer.is_empty() {
                return;
            }
            self.options.extend(buffer.drain(..));
        }
        self.force_render(gs);
    }

    fn resize_success(&mut self, _: &mut GlobalState) -> bool {
        true
    }

    fn paste_passthrough(&mut self, clip: String, _: &mut Components) -> bool {
        self.pattern.paste_passthrough(clip)
    }
}

fn build_path_line((path, ..): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{}", path.display()));
}

fn build_text_line((.., line_txt, line_idx): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{line_idx}| "));
    builder.push(line_txt);
}
