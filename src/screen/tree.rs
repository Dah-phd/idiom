use std::{
    fs::DirEntry,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::state::State;

// #[derive(Clone, Debug)]
// struct Location {
//     path: Path,
//     name: String
// }

pub fn file_tree(terminal: &mut Frame<impl Backend>, state: Arc<RwLock<State>>) {
    let mut tree = if let Some(tree) = &state.read().expect("should not lock!").file_tree {
        tree.clone()
    } else {
        return;
    };
    let mut buffer = vec![];
    for path in std::fs::read_dir("./").unwrap().flatten() {
        expand(path, &mut buffer, &tree.expanded)
    }

    let list: Vec<&str> = buffer.iter().flat_map(|data| data.as_os_str().to_str()).collect();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(15), Constraint::Min(10)].as_ref())
        .split(terminal.size());
    let tasks: Vec<ListItem> = list
        .iter()
        .map(|i| ListItem::new(vec![Spans::from(Span::raw(*i))]))
        .collect();
    let tasks = List::new(tasks)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    let mut state = state.write().expect("should not lock!");
    if let Some(tree) = &mut state.file_tree {
        tree.tree = buffer.clone();
    }
    drop(state);
    terminal.render_stateful_widget(tasks, chunks[0], &mut tree.state);
}

fn expand(path: DirEntry, buffer: &mut Vec<PathBuf>, expansions: &Vec<PathBuf>) {
    let str_pth = String::from(path.path().as_os_str().to_str().unwrap());
    if str_pth.starts_with("./.") || str_pth.starts_with("./target") {
        return;
    }
    buffer.push(path.path());
    if path.path().is_dir() && expansions.contains(&path.path()) {
        for nested in std::fs::read_dir(path.path()).unwrap().flatten() {
            expand(nested, buffer, expansions)
        }
    }
}
