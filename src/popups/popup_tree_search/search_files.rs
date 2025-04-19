use super::{Components, Popup, Status, WAIT_ON_UPDATE};
use crate::{
    embeded_term::EditorTerminal,
    global_state::{GlobalState, IdiomEvent},
    render::{
        backend::StyleExt,
        layout::{LineBuilder, Rect, BORDERS},
        state::State,
        TextField,
    },
    tree::{Tree, TreePath},
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::{Color, ContentStyle};
use std::time::Instant;
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::Mutex as AsyncMutex, task::JoinHandle};

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
    SearchMode { title: " File search (Selected - Tab to switch to Full mode) ", fg_color: Color::Yellow };

const FULL_SEARCH_TITLE: SearchMode = SearchMode { title: " File search (Full) ", fg_color: Color::Red };

pub struct ActiveFileSearch {
    join_handle: Option<JoinHandle<()>>,
    options: Vec<SearchResult>,
    option_buffer: Arc<AsyncMutex<Vec<SearchResult>>>,
    state: State,
    mode: SearchMode,
    pattern: TextField<bool>,
    clock: Option<Instant>,
    tree: TreePath,
}

impl ActiveFileSearch {
    pub fn run(pattern: String, gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let clock = if pattern.len() > 2 { Some(Instant::now()) } else { None };
        let mut new = Self {
            mode: FILE_SEARCH_TITLE,
            join_handle: None,
            option_buffer: Arc::default(),
            options: Vec::default(),
            state: State::default(),
            pattern: TextField::new(pattern, Some(true)),
            clock,
            tree: tree.shallow_copy_selected_tree_path(),
        };

        if new.pattern.text.len() > 1 {
            new.collect_data();
        }

        new.run(gs, ws, tree, term);
    }

    fn collect_data(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            handle.abort();
        }

        self.options.clear();
        self.state.reset();

        if self.pattern.text.len() < 2 {
            return;
        };

        self.clock = Some(Instant::now());
    }

    fn get_rect(gs: &GlobalState) -> Rect {
        gs.screen_rect.center(20, 120).with_borders()
    }

    fn get_option_idx(&self, row: u16, column: u16, gs: &GlobalState) -> Option<usize> {
        let mut rect = Self::get_rect(gs);
        rect.height = rect.height.checked_sub(2)?;
        rect.row += 2;
        let position = rect.relative_position(row, column)?;
        let idx = self.state.at_line + position.line;
        if idx >= self.options.len() {
            return None;
        }
        Some(idx)
    }
}

impl Popup for ActiveFileSearch {
    type R = ();

    fn force_render(&mut self, gs: &mut GlobalState) {
        let mut rect = Self::get_rect(gs);
        let backend = gs.backend();
        rect.draw_borders(None, None, backend);
        rect.border_title_styled(self.mode.title, ContentStyle::fg(self.mode.fg_color), backend);
        let Some(line) = rect.next_line() else { return };
        self.pattern.widget(line, backend);
        let Some(line) = rect.next_line() else { return };
        line.fill(BORDERS.horizontal_top, backend);

        if self.options.is_empty() {
            self.state.render_list(["No results found!"].into_iter(), rect, backend);
        } else {
            self.state.render_list_complex(&self.options, &[build_path_line, build_text_line], rect, backend);
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
            KeyCode::Up => self.state.prev(self.options.len()),
            KeyCode::Down => self.state.next(self.options.len()),
            KeyCode::Tab => {
                if self.mode.is_full() {
                    return Status::Dropped;
                }
                self.mode = FULL_SEARCH_TITLE;
                self.tree = tree.shallow_copy_root_tree_path();
                self.collect_data();
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
        let Components { gs, .. } = components;
        match event {
            MouseEvent { kind: MouseEventKind::Moved, column, row, .. } => match self.get_option_idx(row, column, gs) {
                Some(idx) => self.state.select(idx, self.options.len()),
                None => return Status::Pending,
            },
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => {
                if let Some(index) = self.get_option_idx(row, column, gs) {
                    let (path, _, line) = self.options.remove(index);
                    gs.event.push(IdiomEvent::OpenAtLine(path, line));
                    return Status::Dropped;
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
        match self.clock {
            Some(clock) => {
                if clock.elapsed() < WAIT_ON_UPDATE {
                    return;
                }
                self.clock = None;
                self.options.clear();
                let tree_path = self.tree.clone();
                let buffer = Arc::clone(&self.option_buffer);
                let pattern = self.pattern.text.to_owned();
                self.join_handle.replace(tokio::task::spawn(async move {
                    buffer.lock().await.clear();
                    let mut join_set = tree_path.search_files_join_set(pattern);
                    while let Some(task_result) = join_set.join_next().await {
                        if let Ok(result) = task_result {
                            buffer.lock().await.extend(result);
                        };
                    }
                }));
            }
            None => {
                let Some(handle) = self.join_handle.take() else { return };
                if !handle.is_finished() {
                    self.join_handle = Some(handle);
                }

                // if handle is finished there should not be anything preventing lock
                let Ok(mut buffer) = self.option_buffer.try_lock() else {
                    return;
                };
                if buffer.is_empty() {
                    return;
                }
                self.options.extend(buffer.drain(..));
                drop(buffer);
                self.force_render(gs);
            }
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
