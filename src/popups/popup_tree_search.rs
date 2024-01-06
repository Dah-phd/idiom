use super::PopupInterface;
use crate::{
    global_state::{Clipboard, PopupMessage, TreeEvent},
    tree::Tree,
    widgests::centered_rect_static,
    widgests::{TextField, WrappedState},
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

const SELECTOR_CONSTRAINTS: [Constraint; 2] = [Constraint::Min(3), Constraint::Percentage(100)];

pub struct ActivePathSearch {
    options: Vec<PathBuf>,
    state: WrappedState,
    pattern: TextField<PopupMessage>,
}

impl ActivePathSearch {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            options: Vec::new(),
            state: WrappedState::default(),
            pattern: TextField::with_tree_access(String::new()),
        })
    }
}

impl PopupInterface for ActivePathSearch {
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        if let Some(msg) = self.pattern.map(key, clipboard) {
            return msg;
        }
        match key.code {
            KeyCode::Up => self.state.prev(&self.options),
            KeyCode::Down => self.state.next(&self.options),
            KeyCode::Tab => return PopupMessage::Tree(TreeEvent::SearchFiles(self.pattern.text.to_owned())),
            KeyCode::Enter => {
                return match self.state.selected() {
                    Some(idx) if !self.options.is_empty() => TreeEvent::Open(self.options.remove(idx)).into(),
                    _ => PopupMessage::Clear,
                }
            }
            _ => {}
        }
        PopupMessage::None
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = centered_rect_static(120, 20, frame.size());
        frame.render_widget(Clear, area);
        let split_areas = Layout::new(Direction::Vertical, SELECTOR_CONSTRAINTS).split(area);
        frame.render_widget(
            self.pattern.widget().block(
                Block::new()
                    .borders(Borders::ALL)
                    .title(" Path search (Tab to switch to in File search) ")
                    .title_style(Style { fg: Some(Color::LightBlue), ..Default::default() }),
            ),
            split_areas[0],
        );

        let options = if self.options.is_empty() {
            vec![ListItem::new("No results found!")]
        } else {
            self.options
                .iter()
                .map(|el| ListItem::new(marked_pat_span(&el.display().to_string(), &self.pattern.text)))
                .collect::<Vec<_>>()
        };
        let list = List::new(options)
            .block(Block::new().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, split_areas[1], self.state.get());
    }

    fn update_tree(&mut self, file_tree: &mut Tree) {
        if self.pattern.text.is_empty() {
            self.options.clear();
        } else {
            self.options = file_tree.search_paths(&self.pattern.text);
        };
        self.state.drop();
    }
}

enum Mode {
    Full,
    Select,
}

pub struct ActiveFileSearch {
    join_handle: Option<JoinHandle<()>>,
    options: Vec<(PathBuf, String, usize)>,
    option_buffer: Arc<Mutex<Vec<(PathBuf, String, usize)>>>,
    state: WrappedState,
    mode: Mode,
    pattern: TextField<PopupMessage>,
}

impl ActiveFileSearch {
    pub fn new(pattern: String) -> Box<Self> {
        Box::new(Self {
            mode: Mode::Select,
            join_handle: None,
            option_buffer: Arc::default(),
            options: Vec::default(),
            state: WrappedState::default(),
            pattern: TextField::with_tree_access(pattern),
        })
    }
}

impl PopupInterface for ActiveFileSearch {
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        if let Some(msg) = self.pattern.map(key, clipboard) {
            return msg;
        }
        match key.code {
            KeyCode::Up => self.state.prev(&self.options),
            KeyCode::Down => self.state.next(&self.options),
            KeyCode::Tab => {
                if matches!(self.mode, Mode::Full) {
                    return PopupMessage::Clear;
                }
                self.mode = Mode::Full;
                return PopupMessage::Tree(TreeEvent::PopupAccess);
            }
            KeyCode::Enter => {
                return match self.state.selected() {
                    Some(idx) if !self.options.is_empty() => {
                        let (path, _, line) = self.options.remove(idx);
                        TreeEvent::OpenAtLine(path, line).into()
                    }
                    _ => PopupMessage::Clear,
                }
            }
            _ => {}
        }
        PopupMessage::None
    }

    fn render(&mut self, frame: &mut Frame) {
        if let Ok(mut buffer) = self.option_buffer.try_lock() {
            self.options.extend(buffer.drain(..));
        }
        let area = centered_rect_static(120, 20, frame.size());
        frame.render_widget(Clear, area);
        let split_areas = Layout::new(Direction::Vertical, SELECTOR_CONSTRAINTS).split(area);
        let block = match self.mode {
            Mode::Full => Block::new()
                .borders(Borders::ALL)
                .title(" File search (Full) ")
                .title_style(Style { fg: Some(Color::Red), ..Default::default() }),
            Mode::Select => Block::new()
                .borders(Borders::ALL)
                .title(" File search (Selected - Tab to switch to Full mode) ")
                .title_style(Style { fg: Some(Color::LightYellow), ..Default::default() }),
        };
        frame.render_widget(self.pattern.widget().block(block), split_areas[0]);

        let options = if self.options.is_empty() {
            vec![ListItem::new("No results found!")]
        } else {
            self.options.iter().map(|el| ListItem::new(marked_pat_lines(el, &self.pattern.text))).collect::<Vec<_>>()
        };
        let list = List::new(options)
            .block(Block::new().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, split_areas[1], self.state.get());
    }

    fn update_tree(&mut self, file_tree: &mut Tree) {
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
}

fn marked_pat_lines(option: &(PathBuf, String, usize), pat: &str) -> Vec<Line<'static>> {
    let mut found_text_line = marked_pat_span(option.1.as_str(), pat);
    found_text_line.spans.insert(0, Span::raw(format!("{}| ", option.2)));
    vec![Line::from(format!("{}", option.0.display())), found_text_line]
}

fn marked_pat_span(option: &str, pat: &str) -> Line<'static> {
    let mut v = Vec::new();
    let mut from = 0;
    for (idx, _) in option.match_indices(pat) {
        v.push(Span::styled(option[from..idx].to_owned(), Style { add_modifier: Modifier::DIM, ..Default::default() }));
        from = idx + pat.len();
        v.push(Span::styled(
            option[idx..from].to_owned(),
            Style { add_modifier: Modifier::BOLD, ..Default::default() },
        ));
    }
    v.push(Span::styled(option[from..].to_owned(), Style { add_modifier: Modifier::DIM, ..Default::default() }));
    Line::from(v)
}
