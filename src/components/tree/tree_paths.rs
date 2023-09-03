use crate::utils::get_nested_paths;
use anyhow::{anyhow, Result};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use tui::{
    style::{Color, Modifier, Style},
    widgets::ListItem,
};

#[cfg(not(target_os = "windows"))]
const DIR_SEP: char = '/';

#[cfg(target_os = "windows")]
const DIR_SEP: char = '\\';

#[derive(Debug)]
pub enum TreePath {
    Folder { path: PathBuf, parent: *mut Self, tree: Option<Vec<TreePath>>, errors: usize, warnings: usize },
    File { path: PathBuf, parent: *mut Self, errors: usize, warnings: usize },
}

impl TreePath {
    pub fn as_widgets(&self, selected_path: &Option<PathBuf>) -> Vec<ListItem<'_>> {
        let mut buffer = vec![self.as_list_item(selected_path)];
        if let Self::Folder { tree: Some(tree), .. } = self {
            for tree_element in tree {
                buffer.extend(tree_element.as_widgets(selected_path));
            }
        }
        buffer
    }

    pub fn with_parent(path: PathBuf, parent: *mut Self) -> Self {
        if path.is_dir() {
            return Self::Folder { path, parent, tree: None, errors: 0, warnings: 0 };
        }
        Self::File { path, parent, errors: 0, warnings: 0 }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            Self::File { path, .. } => path,
            Self::Folder { path, .. } => path,
        }
    }

    pub fn path_mut(&mut self) -> &mut PathBuf {
        match self {
            Self::File { path, .. } => path,
            Self::Folder { path, .. } => path,
        }
    }

    pub fn tree(&mut self) -> Option<&mut Vec<Self>> {
        if let Self::Folder { tree: Some(tree), .. } = self {
            return Some(tree);
        }
        None
    }

    pub fn errors(&self) -> usize {
        match self {
            Self::File { errors, .. } => *errors,
            Self::Folder { errors, .. } => *errors,
        }
    }

    pub fn warnings(&self) -> usize {
        match self {
            Self::File { warnings, .. } => *warnings,
            Self::Folder { warnings, .. } => *warnings,
        }
    }

    pub fn new_file(&mut self, name: String) -> Result<PathBuf> {
        let mut new_file = self.path().clone();
        new_file.push(name);
        if !new_file.exists() {
            std::fs::write(&new_file, "")?;
            self.push(new_file.clone());
            return Ok(new_file);
        }
        Err(anyhow!("File already exists! {:?}", new_file))
    }

    fn push(&mut self, path: PathBuf) {
        let ptr: *mut Self = self;
        self.expand();
        if let Self::Folder { tree: Some(tree), .. } = self {
            tree.push(TreePath::with_parent(path, ptr));
            tree.sort_by(order_tree_path);
        };
    }

    pub fn parent(&mut self) -> Option<&mut Self> {
        unsafe {
            match self {
                Self::File { parent, .. } => parent.as_mut(),
                Self::Folder { parent, .. } => parent.as_mut(),
            }
        }
    }

    pub fn refresh(&mut self) {
        let ptr: *mut Self = self;
        if let Self::Folder { path, tree: Some(tree), .. } = self {
            let updated_tree = get_nested_paths(path).collect::<Vec<_>>();
            for path in updated_tree.iter() {
                if !tree.iter().any(|tree_element| tree_element.path() == path) {
                    tree.push(TreePath::with_parent(path.clone(), ptr))
                }
            }
            tree.retain_mut(|el| {
                if updated_tree.contains(el.path()) {
                    el.refresh();
                    return true;
                }
                false
            });
            tree.sort_by(order_tree_path)
        }
    }

    pub fn is_parent(&self, path: &Path) -> bool {
        path.starts_with(self.path())
    }

    pub fn next(&mut self) -> Option<*mut Self> {
        if let Some(tree) = self.tree() {
            if let Some(first) = tree.first_mut() {
                return Some(first);
            }
        }
        self.next_in_parent()
    }

    fn next_in_parent(&mut self) -> Option<*mut Self> {
        let path = self.path().clone();
        if let Some(parent) = self.parent() {
            if let Some(tree) = parent.tree() {
                let mut return_next = false;
                for tree_path in tree {
                    if return_next {
                        return Some(tree_path);
                    }
                    if tree_path.path() == &path {
                        return_next = true;
                    }
                }
            }
            return parent.next_in_parent();
        }
        None
    }

    pub fn prev(&mut self) -> Option<*mut Self> {
        let path = self.path().clone();
        if let Some(parent) = self.parent() {
            let ptr: *mut TreePath = parent;
            if let Some(tree) = parent.tree() {
                let mut return_next = false;
                for tree_path in tree.iter_mut().rev() {
                    if return_next {
                        return tree_path.last_child();
                    }
                    if tree_path.path() == &path {
                        return_next = true;
                    }
                }
            }
            return Some(ptr);
        }
        None
    }

    fn last_child(&mut self) -> Option<*mut Self> {
        if let Some(tree) = self.tree() {
            if let Some(last) = tree.last_mut() {
                return last.last_child();
            }
        }
        Some(self)
    }

    pub fn expand(&mut self) {
        let parent: *mut Self = self;
        if let Self::Folder { path, tree, .. } = self {
            if tree.is_some() {
                return;
            }
            let mut nested_tree = get_nested_paths(path).map(|p| Self::with_parent(p, parent)).collect::<Vec<_>>();
            nested_tree.sort_by(order_tree_path);
            *tree = Some(nested_tree);
        }
    }

    pub fn as_list_item(&self, selected_path: &Option<PathBuf>) -> ListItem {
        let path_str = &self.path().as_path().display().to_string()[2..];
        let mut buffer = String::new();
        let mut path_split = path_str.split(DIR_SEP).peekable();
        while let Some(path_element) = path_split.next() {
            if path_split.peek().is_none() {
                buffer.push_str(path_element)
            } else {
                buffer.push_str("  ")
            }
        }
        if self.path().is_dir() {
            buffer.push_str("/..");
        }
        let mut style = Style::default();
        if matches!(selected_path, Some(path) if path == self.path()) {
            style = style.add_modifier(Modifier::REVERSED);
        }
        let errors = self.errors();
        let warnings = self.warnings();
        if errors != 0 {
            style = style.fg(Color::Red);
            buffer.push_str(&format!(" {}", errors));
        } else if warnings != 0 {
            style = style.fg(Color::Yellow);
            buffer.push_str(&format!(" {}", warnings));
        }
        ListItem::new(buffer).style(style)
    }
}

pub fn order_tree_path(left: &TreePath, right: &TreePath) -> Ordering {
    match (matches!(left, TreePath::Folder { .. }), matches!(right, TreePath::Folder { .. })) {
        (true, true) => Ordering::Equal,
        (false, false) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}
