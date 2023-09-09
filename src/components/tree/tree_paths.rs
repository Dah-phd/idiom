use crate::utils::get_nested_paths;
use std::{
    cmp::Ordering,
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub enum TreePath {
    Folder { path: PathBuf, tree: Option<Vec<TreePath>>, display: String },
    File { path: PathBuf, display: String },
}

impl Default for TreePath {
    fn default() -> Self {
        let path = PathBuf::from("./");
        let mut tree_buffer = get_nested_paths(&path).map(|p| p.into()).collect::<Vec<Self>>();
        tree_buffer.sort_by(order_tree_path);
        Self::Folder { display: get_path_display(&path), path, tree: Some(tree_buffer) }
    }
}

impl From<PathBuf> for TreePath {
    fn from(value: PathBuf) -> Self {
        let display = get_path_display(&value);
        if value.is_dir() {
            Self::Folder { path: value, tree: None, display }
        } else {
            Self::File { path: value, display }
        }
    }
}

impl TreePath {
    pub fn sync_flat_ptrs(&mut self, buffer: &mut Vec<*mut Self>, ignore_base_paths: &[PathBuf]) {
        buffer.clear();
        if let Some(base_tree) = self.tree_mut() {
            for base_path in base_tree {
                if !ignore_base_paths.contains(base_path.path()) {
                    base_path.fill_flat_ptrs(buffer);
                }
            }
        }
    }

    fn fill_flat_ptrs(&mut self, buffer: &mut Vec<*mut Self>) {
        buffer.push(self);
        if let Some(tree) = self.tree_mut() {
            for tree_path in tree {
                tree_path.fill_flat_ptrs(buffer);
            }
        }
    }

    pub fn expand(&mut self) {
        if let Self::Folder { tree, path, .. } = self {
            if tree.is_some() {
                return;
            }
            let mut buffer = Vec::new();
            for nested_path in get_nested_paths(path) {
                buffer.push(nested_path.into())
            }
            buffer.sort_by(order_tree_path);
            tree.replace(buffer);
        }
    }

    pub fn expand_contained(&mut self, path: &PathBuf) -> bool {
        if self.path() == path {
            return true;
        }
        if path.starts_with(self.path()) {
            let should_shrink = self.tree_mut().is_none();
            self.expand();
            if let Some(nested_tree) = self.tree_mut() {
                for tree_path in nested_tree {
                    if tree_path.expand_contained(path) {
                        return true;
                    }
                }
            }
            if should_shrink {
                let _ = self.take_tree();
            }
        }
        false
    }

    pub fn sync(&mut self) {
        if let Self::Folder { path, tree: Some(tree), .. } = self {
            let updated_tree = get_nested_paths(path).collect::<HashSet<_>>();
            for path in updated_tree.iter() {
                if !tree.iter().any(|tree_element| tree_element.path() == path) {
                    tree.push(path.clone().into())
                }
            }
            tree.retain_mut(|tree_path| {
                if updated_tree.contains(tree_path.path()) {
                    tree_path.sync();
                    return true;
                }
                false
            });
            tree.sort_by(order_tree_path)
        }
    }

    pub fn display(&self) -> &str {
        match self {
            Self::Folder { display, .. } => display,
            Self::File { display, .. } => display,
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Folder { path, .. } => path,
            Self::File { path, .. } => path,
        }
    }

    pub fn update_path(&mut self, new_path: PathBuf) {
        match self {
            Self::File { path, display } => {
                *display = get_path_display(&new_path);
                *path = new_path;
            }
            Self::Folder { path, display, .. } => {
                *display = get_path_display(&new_path);
                *path = new_path;
            }
        }
    }

    pub fn take_tree(&mut self) -> Option<Vec<Self>> {
        if let Self::Folder { tree, .. } = self {
            return tree.take();
        }
        None
    }

    fn tree_mut(&mut self) -> Option<&mut Vec<TreePath>> {
        if let Self::Folder { tree: Some(tree), .. } = self {
            return Some(tree);
        }
        None
    }
}

fn order_tree_path(left: &TreePath, right: &TreePath) -> Ordering {
    match (matches!(left, TreePath::Folder { .. }), matches!(right, TreePath::Folder { .. })) {
        (true, true) => Ordering::Equal,
        (false, false) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}

fn get_path_display(path: &Path) -> String {
    let path_str = &path.display().to_string()[2..];
    let mut buffer = String::new();
    let mut path_split = path_str.split(std::path::MAIN_SEPARATOR).peekable();
    while let Some(path_element) = path_split.next() {
        if path_split.peek().is_none() {
            buffer.push_str(path_element)
        } else {
            buffer.push_str("  ")
        }
    }
    if path.is_dir() {
        buffer.push_str("/..");
    }
    buffer
}
