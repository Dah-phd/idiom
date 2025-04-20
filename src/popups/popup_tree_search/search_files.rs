use super::{Components, Popup, Status};
use crate::{
    embeded_term::EditorTerminal,
    global_state::GlobalState,
    render::{
        backend::StyleExt,
        layout::{LineBuilder, Rect, BORDERS},
        state::State,
        TextField,
    },
    tree::{Tree, TreePath},
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::Color;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

const MAX_SEARCH_TIME: Duration = Duration::from_millis(50);

type SearchResult = (PathBuf, String, usize);

struct SearchMode {
    title: &'static str,
    fg_color: Color,
}

impl SearchMode {
    fn is_full(&self) -> bool {
        self.fg_color == Color::Red
    }
}

const FILE_SEARCH_TITLE: SearchMode =
    SearchMode { title: "File search (Selected - Tab for root search)", fg_color: Color::Yellow };

const FULL_SEARCH_TITLE: SearchMode = SearchMode { title: "File search (root)", fg_color: Color::Red };

struct CachedBuffer {
    base_search: String,
    filter: String,
    base_line: Vec<SearchResult>,
    tasks: Option<JoinSet<Vec<SearchResult>>>,
    tree: TreePath,
}

impl CachedBuffer {
    fn new(tree: TreePath) -> Self {
        Self { base_search: String::new(), filter: String::new(), base_line: Vec::new(), tasks: None, tree }
    }

    fn rebase_cache(&mut self) {
        _ = self.tasks.take().map(|mut t| t.abort_all());
        self.tasks = Some(self.tree.clone().search_files_join_set(self.base_search.to_owned()));
    }

    fn len(&self) -> usize {
        if self.filter.is_empty() {
            return self.base_line.len();
        }
        self.base_line.iter().filter(|res| res.1.contains(self.filter.as_str())).count()
    }

    fn is_empty(&self) -> bool {
        if self.filter.is_empty() {
            return self.base_line.is_empty();
        }
        !self.base_line.iter().any(|res| res.1.contains(self.filter.as_str()))
    }

    fn vec(&mut self) -> Vec<SearchResult> {
        self.flush_results();
        self.base_line.iter().filter(|res| res.1.contains(self.filter.as_str())).cloned().collect()
    }

    fn is_running(&self) -> bool {
        self.tasks.as_ref().map(|t| !t.is_empty()).unwrap_or_default()
    }

    fn set_search(&mut self, pattern: &str) {
        if pattern.len() < 2 {
            _ = self.tasks.take().map(|mut t| t.abort_all());
            self.base_line.clear();
            self.base_search.clear();
            self.filter.clear();
            return;
        }
        if self.base_search.is_empty() || !pattern.starts_with(&self.base_search) {
            self.base_search = pattern.to_owned();
            self.rebase_cache();
        } else {
            self.filter = pattern.to_owned();
        }
    }

    fn new_tree(&mut self, tree: TreePath, pattern: &str) {
        // drop existing tasks
        _ = self.tasks.take().map(|mut t| t.abort_all());
        // replace self with clean buffer
        *self = Self::new(tree);
        self.set_search(pattern);
    }

    fn flush_results(&mut self) {
        let Some(tasks) = self.tasks.as_mut() else { return };
        if tasks.is_empty() {
            self.tasks = None;
            return;
        }
        let start = Instant::now();
        loop {
            if start.elapsed() > MAX_SEARCH_TIME {
                return;
            }
            let Some(result) = tasks.try_join_next() else { return };
            if let Ok(data) = result {
                self.base_line.extend(data);
            }
        }
    }
}

pub struct ActiveFileSearch {
    state: State,
    mode: SearchMode,
    pattern: TextField<bool>,
    buffer: CachedBuffer,
}

impl ActiveFileSearch {
    pub fn run(pattern: String, gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let path_tree = tree.shallow_copy_selected_tree_path();
        let mut buffer = CachedBuffer::new(path_tree);
        if pattern.len() >= 2 {
            buffer.set_search(&pattern);
        }

        let mut new = Self {
            mode: FILE_SEARCH_TITLE,
            state: State::default(),
            pattern: TextField::new(pattern, Some(true)),
            buffer,
        };

        if new.pattern.text.len() > 1 {
            new.collect_data();
        }

        new.run(gs, ws, tree, term);
    }

    fn collect_data(&mut self) {
        self.state.reset();
        self.buffer.set_search(&self.pattern.text);
    }

    fn get_rect(gs: &GlobalState) -> Rect {
        gs.screen_rect.center(20, 120).with_borders()
    }

    fn get_option_idx(&self, row: u16, column: u16, gs: &GlobalState) -> Option<usize> {
        let mut rect = Self::get_rect(gs);
        rect.height = rect.height.checked_sub(2)?;
        rect.row += 2;
        let position = rect.relative_position(row, column)?;
        let idx = (self.state.at_line + position.line) / 2;
        if idx >= self.buffer.len() {
            return None;
        }
        Some(idx)
    }
}

impl Popup for ActiveFileSearch {
    type R = ();

    fn force_render(&mut self, gs: &mut GlobalState) {
        let mut rect = Self::get_rect(gs);
        let accent_style = gs.theme.accent_style.with_fg(self.mode.fg_color);
        let backend = gs.backend();
        rect.draw_borders(None, None, backend);
        rect.border_title_styled(self.mode.title, accent_style, backend);
        let Some(line) = rect.next_line() else { return };
        self.pattern.widget(line, backend);
        let Some(line) = rect.next_line() else { return };
        line.fill(BORDERS.horizontal_top, backend);

        if self.buffer.is_empty() {
            if self.buffer.is_running() {
                self.state.render_list(["Searching ..."].into_iter(), rect, backend);
            } else {
                self.state.render_list(["No results found!"].into_iter(), rect, backend);
            }
        } else {
            self.state.render_list_complex(&self.buffer.vec(), &[build_path_line, build_text_line], rect, backend);
        }
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R> {
        let Components { gs, tree, .. } = components;

        if let Some(updated) = self.pattern.map(&key, &mut gs.clipboard) {
            if updated {
                self.collect_data();
            }
            self.force_render(gs);
            return Status::Pending;
        }
        match key.code {
            KeyCode::Up => self.state.prev(self.buffer.len()),
            KeyCode::Down => self.state.next(self.buffer.len()),
            KeyCode::Tab => {
                if self.mode.is_full() {
                    return Status::Dropped;
                }
                self.mode = FULL_SEARCH_TITLE;
                self.buffer.new_tree(tree.shallow_copy_root_tree_path(), &self.pattern.text);
                self.collect_data();
            }
            KeyCode::Enter => {
                if self.buffer.len() > self.state.selected {
                    todo!()
                    // let (path, _, line) = self.options.remove(self.state.selected);
                    // gs.event.push(IdiomEvent::OpenAtLine(path, line));
                }
                return Status::Dropped;
            }
            _ => return Status::Pending,
        }
        self.force_render(gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status<Self::R> {
        let Components { gs, .. } = components;
        match event {
            MouseEvent { kind: MouseEventKind::Moved, column, row, .. } => match self.get_option_idx(row, column, gs) {
                Some(idx) => self.state.select(idx, self.buffer.len()),
                None => return Status::Pending,
            },
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => {
                if let Some(index) = self.get_option_idx(row, column, gs) {
                    todo!()
                    // let (path, _, line) = self.buffer.remove(index);
                    // gs.event.push(IdiomEvent::OpenAtLine(path, line));
                    // return Status::Dropped;
                }
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => self.state.prev(self.buffer.len()),
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => self.state.next(self.buffer.len()),
            _ => return Status::Pending,
        }
        self.force_render(gs);
        Status::Pending
    }

    fn render(&mut self, gs: &mut GlobalState) {
        if self.buffer.is_running() {
            self.buffer.flush_results();
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

fn build_path_line((path, ..): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{}", path.display()));
}

fn build_text_line((.., line_txt, line_idx): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{line_idx}| "));
    builder.push(line_txt);
}
