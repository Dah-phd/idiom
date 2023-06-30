use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

#[cfg(not(target_os = "windows"))]
const DIR_SEP: char = '/';

#[cfg(target_os = "windows")]
const DIR_SEP: char = '\\';

#[derive(Clone, Default)]
pub struct Tree {
    pub expanded: Vec<PathBuf>,
    pub state: ListState,
    pub tree: Vec<PathBuf>,
    pub on_open_tabs: bool,
    pub input: Option<String>,
}

impl Tree {
    pub fn render(&mut self, frame: &mut Frame<impl Backend>, area: Rect) {
        let mut tree = TreePath::default();
        tree.expand(&self.expanded);
        self.tree = tree.flatten();
        let str_paths: Vec<ListItem<'_>> = self.tree.iter().map(stringify_path).collect();

        let file_tree = List::new(str_paths)
            .block(Block::default().borders(Borders::ALL).title("Explorer"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(file_tree, area, &mut self.state);
    }

    pub fn expand_dir_or_get_path(&mut self) -> Option<PathBuf> {
        let path = self.get_path()?;
        if path.is_dir() {
            self.expanded.push(path);
            None
        } else {
            Some(path)
        }
    }

    fn get_path(&mut self) -> Option<PathBuf> {
        Some(self.tree[self.state.selected()?].clone())
    }

    pub fn enter_file_name(&mut self) {
        let path = self.state.selected();
    }

    pub fn rename(&mut self) -> Option<PathBuf> {
        let new_name = self.input.take()?;
        std::fs::rename(self.get_path()?, &new_name).ok()?;
        Some(PathBuf::from(new_name))
    }

    fn create_file(&mut self) {}

    fn delete_file(&mut self) {}

    pub fn map(&mut self, key: &KeyEvent) -> bool {
        match key.modifiers {
            KeyModifiers::NONE => match key.code {
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.on_open_tabs = false;
                    if let Some(numba) = self.state.selected() {
                        if numba == 0 {
                            self.state.select(Some(self.tree.len() - 1))
                        } else {
                            self.state.select(Some(numba - 1))
                        }
                    } else {
                        self.state.select(Some(self.tree.len() - 1))
                    }
                }
                KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.on_open_tabs = false;
                    if let Some(numba) = self.state.selected() {
                        if numba < self.tree.len() - 1 {
                            self.state.select(Some(numba + 1));
                        } else {
                            self.state.select(Some(0))
                        }
                    } else {
                        self.state.select(Some(0))
                    }
                }
                KeyCode::Left => {
                    if let Some(numba) = self.state.selected() {
                        if let Some(path) = self.tree.get(numba) {
                            self.expanded.retain(|expanded_path| expanded_path != path)
                        }
                    }
                }
                _ => return false,
            },
            KeyModifiers::CONTROL => match key.code {
                KeyCode::Char('n') | KeyCode::Char('N') => panic!("force quit!"),
                _ => return false,
            },
            _ => return false,
        }
        true
    }
}

#[derive(Debug, Clone)]
pub enum TreePath {
    Folder { path: PathBuf, tree: Option<Vec<TreePath>> },
    File(PathBuf),
}

impl Default for TreePath {
    fn default() -> Self {
        let path = PathBuf::from("./");
        let mut tree = vec![];
        for path in std::fs::read_dir(&path).unwrap().flatten() {
            let path = path.path();
            if path.is_dir() {
                tree.push(Self::Folder { path, tree: None })
            } else {
                tree.push(Self::File(path))
            }
        }
        Self::Folder { path, tree: Some(tree) }
    }
}

impl TreePath {
    fn probe(&mut self) {
        (*self) = Self::default();
    }

    fn flatten(self) -> Vec<PathBuf> {
        let mut buffer = Vec::new();
        match self {
            Self::File(path) => buffer.push(path),
            Self::Folder { path, tree } => {
                buffer.push(path);
                if let Some(nester_tree) = tree {
                    for tree_element in nester_tree {
                        buffer.extend(tree_element.flatten());
                    }
                }
            }
        }
        buffer
    }

    fn len(&self) -> usize {
        match self {
            Self::File(_) => 1,
            Self::Folder { path, tree } => {
                let mut len = if path == &PathBuf::from("./") { 0 } else { 1 };
                if let Some(nested_tree) = tree {
                    for tree_element in nested_tree {
                        len += tree_element.len();
                    }
                }
                len
            }
        }
    }

    fn expand(&mut self, expanded: &Vec<PathBuf>) {
        if let Self::Folder { path, tree } = self {
            if let Some(tree) = tree {
                for nested in tree {
                    nested.expand(expanded);
                }
            } else if expanded.contains(path) {
                let mut tree_buffer = vec![];
                for nested in std::fs::read_dir(path).unwrap().flatten() {
                    let path = nested.path();
                    if path.is_dir() {
                        let mut folder = Self::Folder { path, tree: None };
                        folder.expand(expanded);
                        tree_buffer.push(folder)
                    } else {
                        tree_buffer.push(Self::File(path))
                    }
                }
                (*tree) = Some(tree_buffer);
            }
        }
    }
}

#[allow(clippy::ptr_arg)]
fn stringify_path(current_path: &PathBuf) -> ListItem<'static> {
    let path_str = &current_path.as_path().display().to_string()[2..];
    let mut buffer = String::new();
    let mut path_split = path_str.split(DIR_SEP).peekable();
    while let Some(path_element) = path_split.next() {
        if path_split.peek().is_none() {
            buffer.push_str(path_element)
        } else {
            buffer.push_str("  ")
        }
    }
    if current_path.is_dir() {
        buffer.push_str("/..");
    }
    ListItem::new(buffer)
}
