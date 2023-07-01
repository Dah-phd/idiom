use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{cmp::Ordering, path::PathBuf};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
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
        self.tree.retain(not_expcluded_path);

        let mut paths: Vec<ListItem<'_>> = self.tree.iter().map(path_to_list_item).collect();
        self.inject_input_if_exists(&mut paths);

        let file_tree = List::new(paths)
            .block(Block::default().borders(Borders::ALL).title("Explorer"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(file_tree, area, &mut self.state);
    }

    pub fn expand_dir_or_get_path(&mut self) -> Option<PathBuf> {
        let path = self.tree.get(self.state.selected()?)?.clone();
        if path.is_dir() {
            self.expanded.push(path);
            None
        } else {
            Some(path)
        }
    }

    fn inject_input_if_exists(&self, paths: &mut Vec<ListItem<'_>>) -> Option<()> {
        if let Some(input) = &self.input {
            let numba = self.state.selected()?;
            if numba == 0 {
                paths.insert(numba, ListItem::new(input.to_owned()))
            } else {
                let mut base_element = self.tree[numba - 1].clone();
                base_element.push(input);
                paths.insert(numba, path_to_list_item(&base_element));
            }
        }
        Some(())
    }

    pub fn rename(&mut self) -> Option<()> {
        let new_name = self.input.take()?;
        std::fs::rename(&self.tree[self.state.selected()?], new_name).ok()?;
        Some(())
    }

    fn create_new(&mut self) -> Option<PathBuf> {
        let numba = self.state.selected()?;
        let mut new_file = if numba == 0 {
            PathBuf::from("./")
        } else {
            get_dir_path(&self.tree[numba - 1])
        };
        let name = self.input.take()?;
        new_file.push(&name);
        if new_file.exists() || std::fs::write(&new_file, "").is_err() {
            self.input.replace(name);
            return None;
        }
        Some(new_file)
    }

    fn delete_file(&mut self) -> Option<()> {
        let numba = self.state.selected()?;
        let path = &self.tree[numba];
        if path.is_file() {
            std::fs::remove_file(path)
        } else {
            std::fs::remove_dir_all(path)
        }
        .ok()
    }

    fn map_tree(&mut self, key: &KeyEvent) -> bool {
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
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.input = Some(String::new());
                    if let Some(numba) = self.state.selected() {
                        self.state.select(Some(numba + 1));
                    } else {
                        self.state.select(Some(0));
                    }
                }
                _ => return false,
            },
            KeyModifiers::SHIFT => match key.code {
                KeyCode::Delete => {
                    self.delete_file();
                }
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    fn map_input(&mut self, key: &KeyEvent) -> bool {
        if let Some(input) = &mut self.input {
            match key.modifiers {
                KeyModifiers::NONE => match key.code {
                    KeyCode::Char(c) => input.push(c),
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Esc => self.input = None,
                    KeyCode::Enter => {
                        if let Some(path) = self.create_new() {
                            if let Some(parent) = path.parent() {
                                if !self.expanded.contains(&PathBuf::from(parent)) {
                                    self.expanded.push(parent.into())
                                }
                            }
                        };
                    }
                    _ => return false,
                },
                _ => return false,
            }
            true
        } else {
            false
        }
    }

    pub fn map(&mut self, key: &KeyEvent) -> bool {
        if self.input.is_none() {
            self.map_tree(key)
        } else {
            self.map_input(key)
        }
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
        let mut tree_buffer = vec![];
        for path in std::fs::read_dir(&path).unwrap().flatten() {
            let path = path.path();
            if path.is_dir() {
                tree_buffer.push(Self::Folder { path, tree: None })
            } else {
                tree_buffer.push(Self::File(path))
            }
        }
        Self::Folder {
            path,
            tree: Some(tree_buffer),
        }
    }
}

impl TreePath {
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

    fn expand(&mut self, expanded: &Vec<PathBuf>) {
        if let Self::Folder { path, tree } = self {
            if let Some(tree) = tree {
                tree.sort_by(order_tree_path);
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
                tree_buffer.sort_by(order_tree_path);
                (*tree) = Some(tree_buffer);
            }
        }
    }
}

#[allow(clippy::ptr_arg)]
fn path_to_list_item(current_path: &PathBuf) -> ListItem<'static> {
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

fn order_tree_path(left: &TreePath, right: &TreePath) -> Ordering {
    match (
        matches!(left, TreePath::Folder { .. }),
        matches!(right, TreePath::Folder { .. }),
    ) {
        (true, true) => Ordering::Equal,
        (false, false) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}

fn not_expcluded_path(path: &PathBuf) -> bool {
    let exclued_files: [PathBuf; 0] = [];
    let excluded_dirs: [PathBuf; 2] = [PathBuf::from("./.git"), PathBuf::from("./")];
    if path.is_dir() {
        !excluded_dirs.contains(path)
    } else {
        !exclued_files.contains(path)
    }
}

#[allow(clippy::ptr_arg)]
fn get_dir_path(path: &PathBuf) -> PathBuf {
    if path.is_file() {
        if let Some(parent) = path.parent() {
            parent.into()
        } else {
            PathBuf::from("./")
        }
    } else {
        path.clone()
    }
}
