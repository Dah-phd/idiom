use std::path::PathBuf;

use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

#[derive(Clone, Default)]
pub struct Tree {
    pub expanded: Vec<PathBuf>,
    pub state: ListState,
    pub tree: Vec<PathBuf>,
}

impl Tree {
    pub fn render(&mut self, frame: &mut Frame<impl Backend>, area: Rect) {
        self.tree.clear();

        for path in std::fs::read_dir("./").unwrap().flatten() {
            self.tree.extend(expand(path.path(), &self.expanded))
        }

        let tasks: Vec<ListItem> = self
            .tree
            .iter()
            .flat_map(use_proper_list_names)
            .map(|path| ListItem::new(vec![Spans::from(Span::raw(path))]))
            .collect();

        let file_tree = List::new(tasks)
            .block(Block::default().borders(Borders::ALL).title("Explorer"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">");

        frame.render_stateful_widget(file_tree, area, &mut self.state);
    }

    pub fn expand_dir_or_get_path(&mut self) -> Option<PathBuf> {
        if let Some(numba) = self.state.selected() {
            if let Some(path) = self.tree.get(numba) {
                if path.is_dir() {
                    self.expanded.push(path.clone())
                } else {
                    return Some(path.clone());
                }
            }
        }
        None
    }
}

fn expand(path: PathBuf, expansions: &Vec<PathBuf>) -> Vec<PathBuf> {
    let mut buffer = vec![path.clone()];
    if path.is_dir() && expansions.contains(&path) {
        for nested in std::fs::read_dir(path).unwrap().flatten() {
            buffer.extend(expand(nested.path(), expansions));
        }
    }
    // TODO ordering
    buffer
}

#[allow(clippy::ptr_arg)]
fn use_proper_list_names(current_path: &PathBuf) -> Option<String> {
    let path_str = current_path.as_os_str().to_str()?;
    let mut buffer = String::new();
    let mut path_split = path_str.split('/').peekable();
    let _ = path_split.next();
    while let Some(path_element) = path_split.next() {
        if path_split.peek().is_none() {
            buffer.push_str(path_element)
        } else {
            buffer.push_str("  ")
        }
    }
    Some(buffer)
}
