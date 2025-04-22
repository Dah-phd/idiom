use super::{Components, Popup, Status};
use crate::{
    embeded_term::EditorTerminal,
    global_state::{GlobalState, IdiomEvent},
    render::{
        backend::{BackendProtocol, StyleExt},
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
use tokio::{
    sync::mpsc::{Receiver, UnboundedSender},
    task::JoinHandle,
};

const MAX_SEARCH_TIME: Duration = Duration::from_millis(50);

const FULL_SEARCH: Color = Color::Red;
const FULL_TITLE: &str = "File search (root)";
const FULL_SEARCH_TITLE: SearchMode = SearchMode { title: FULL_TITLE, fg_color: FULL_SEARCH };

const FILE_SEARCH: Color = Color::Yellow;
const FILE_TITLE: &str = "File search (Selected - Tab for root search)";
const FILE_SEARCH_TITLE: SearchMode = SearchMode { title: FILE_TITLE, fg_color: FILE_SEARCH };

type SearchResult = (PathBuf, String, usize);

struct SearchMode {
    title: &'static str,
    fg_color: Color,
}

impl SearchMode {
    fn is_full(&self) -> bool {
        self.fg_color == FULL_SEARCH
    }
}

enum Message {
    Values(Vec<SearchResult>),
    Reset(Vec<SearchResult>),
    Finished,
}

pub struct ActiveFileSearch {
    options: Vec<SearchResult>,
    state: State,
    mode: SearchMode,
    pattern: TextField<bool>,
    is_searching: bool,
    send: UnboundedSender<String>,
    recv: Receiver<Message>,
    task: JoinHandle<()>,
}

impl ActiveFileSearch {
    pub fn run(pattern: String, gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let (task, send, recv) = create_async_tree_search_task(tree.shallow_copy_selected_tree_path());

        let mut new = Self {
            options: vec![],
            is_searching: false,
            mode: FILE_SEARCH_TITLE,
            state: State::default(),
            pattern: TextField::new(pattern, Some(true)),
            send,
            recv,
            task,
        };

        if let Err(error) = new.collect_data() {
            gs.error(error);
            return;
        };

        new.run(gs, ws, tree, term);
    }

    fn collect_data(&mut self) -> Result<(), tokio::sync::mpsc::error::SendError<String>> {
        self.options.clear();
        self.state.reset();
        if self.pattern.text.len() <= 2 {
            return Ok(());
        }
        self.is_searching = true;
        self.send.send(self.pattern.text.clone())
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
        let accent_style = gs.theme.accent_style.with_fg(self.mode.fg_color);
        let backend = gs.backend();
        backend.freeze();
        rect.draw_borders(None, None, backend);
        rect.border_title_styled(self.mode.title, accent_style, backend);
        let Some(line) = rect.next_line() else { return };
        self.pattern.widget_with_count(line, self.options.len(), backend);
        let Some(line) = rect.next_line() else { return };
        line.fill(BORDERS.horizontal_top, backend);

        if self.options.is_empty() {
            if self.is_searching {
                self.state.render_list(["Searching ..."].into_iter(), rect, backend);
            } else {
                self.state.render_list(["No results found!"].into_iter(), rect, backend);
            }
        } else {
            self.state.render_list_complex(&self.options, &[build_path_line, build_text_line], rect, backend);
        }
        backend.unfreeze();
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R> {
        let Components { gs, tree, .. } = components;

        if let Some(updated) = self.pattern.map(&key, &mut gs.clipboard) {
            if updated {
                if let Err(error) = self.collect_data() {
                    gs.error(error);
                    return Status::Dropped;
                }
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
                self.task.abort();
                let (task, send, recv) = create_async_tree_search_task(tree.shallow_copy_root_tree_path());
                self.task = task;
                self.send = send;
                self.recv = recv;
                if let Err(error) = self.collect_data() {
                    gs.error(error);
                    return Status::Dropped;
                };
            }
            KeyCode::Enter => {
                let (path, _, line) = self.options.remove(self.state.selected);
                gs.event.push(IdiomEvent::OpenAtLine(path, line));
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
        if self.recv.is_empty() {
            return;
        }
        let now = Instant::now();
        while let Ok(msg) = self.recv.try_recv() {
            match msg {
                Message::Reset(new_data) => self.options = new_data,
                Message::Values(new_data) => self.options.extend(new_data),
                Message::Finished => self.is_searching = false,
            }
            if now.elapsed() >= MAX_SEARCH_TIME {
                break;
            }
        }
        self.force_render(gs);
    }

    fn resize_success(&mut self, _: &mut GlobalState) -> bool {
        true
    }

    fn paste_passthrough(&mut self, clip: String, _: &mut Components) -> bool {
        if self.pattern.paste_passthrough(clip) {
            _ = self.collect_data();
            return true;
        }
        false
    }
}

impl Drop for ActiveFileSearch {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn create_async_tree_search_task(tree: TreePath) -> (JoinHandle<()>, UnboundedSender<String>, Receiver<Message>) {
    let (send_results, recv) = tokio::sync::mpsc::channel::<Message>(20);
    let (send, mut recv_requests) = tokio::sync::mpsc::unbounded_channel::<String>();
    let task = tokio::task::spawn(async move {
        let mut cache = (vec![], String::new());
        while let Some(pattern) = recv_requests.recv().await {
            if !cache.0.is_empty() && pattern.starts_with(cache.1.as_str()) {
                if send_results
                    .send(Message::Reset(
                        cache.0.iter().filter(|sr: &&SearchResult| sr.1.contains(pattern.as_str())).cloned().collect(),
                    ))
                    .await
                    .is_err()
                {
                    return;
                };
                if send_results.send(Message::Finished).await.is_err() {
                    return;
                }
                continue;
            }

            let mut buffer = vec![];
            let mut results = tree.clone().search_files_join_set(pattern.to_owned());

            let Some(Ok(first_msg)) = results.join_next().await else {
                continue;
            };
            buffer.extend(first_msg.iter().cloned());
            if send_results.send(Message::Reset(first_msg)).await.is_err() {
                return;
            };

            while let Some(Ok(msg)) = results.join_next().await {
                if !recv_requests.is_empty() {
                    break;
                }
                buffer.extend(msg.iter().cloned());
                if send_results.send(Message::Values(msg)).await.is_err() {
                    return;
                };
            }

            if send_results.send(Message::Finished).await.is_err() {
                return;
            }

            cache = (buffer, pattern);
        }
    });
    (task, send, recv)
}

fn build_path_line((path, ..): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{}", path.display()));
}

fn build_text_line((.., line_txt, line_idx): &SearchResult, mut builder: LineBuilder) {
    builder.push(&format!("{line_idx}| "));
    builder.push(line_txt);
}
