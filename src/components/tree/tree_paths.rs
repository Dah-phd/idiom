use crate::utils::get_nested_paths;
use anyhow::{anyhow, Result};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use tui::{
    style::{Modifier, Style},
    widgets::ListItem,
};

#[cfg(not(target_os = "windows"))]
pub const DIR_SEP: char = '/';

#[cfg(target_os = "windows")]
pub const DIR_SEP: char = '\\';

#[derive(Debug)]
pub enum TreePath {
    Folder { path: PathBuf, parent: *mut Self, tree: Option<Vec<TreePath>>, dispaly: String, style: Style },
    File { path: PathBuf, parent: *mut Self, dispaly: String, style: Style },
}

impl TreePath {
    pub fn with_parent(path: PathBuf, parent: *mut Self) -> Self {
        let dispaly = get_path_display(&path);
        if path.is_dir() {
            return Self::Folder { path, parent, tree: None, dispaly, style: Style::default() };
        }
        Self::File { path, parent, dispaly, style: Style::default() }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            Self::File { path, .. } => path,
            Self::Folder { path, .. } => path,
        }
    }

    pub fn as_widgets(&self) -> Vec<ListItem<'_>> {
        let mut buffer = vec![self.as_list_item()];
        if let Self::Folder { tree: Some(tree), .. } = self {
            for tree_element in tree {
                buffer.extend(tree_element.as_widgets());
            }
        }
        buffer
    }

    pub fn select(&mut self) {
        match self {
            Self::File { style, .. } => {
                style.add_modifier = Modifier::REVERSED;
            }
            Self::Folder { style, .. } => {
                style.add_modifier = Modifier::REVERSED;
            }
        }
    }

    pub fn deselect(&mut self) {
        match self {
            Self::File { style, .. } => {
                style.add_modifier = Modifier::empty();
            }
            Self::Folder { style, .. } => {
                style.add_modifier = Modifier::empty();
            }
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, Self::Folder { .. })
    }

    pub fn rename(&mut self, new_name: &str) -> Option<()> {
        let path_ref = self.path_mut();
        std::fs::rename(&path_ref, new_name).ok()?;
        let new_dispaly = get_path_display(path_ref);
        match self {
            Self::File { dispaly, .. } => *dispaly = new_dispaly,
            Self::Folder { dispaly, .. } => *dispaly = new_dispaly,
        }
        Some(())
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

    pub fn new_file_or_folder(&mut self, name: String) -> Result<PathBuf> {
        self.expand();
        if matches!(self, Self::File { .. }) {
            return Err(anyhow!("Parent is file not a directory!"));
        }
        let result = new_file_or_folder(self.path().clone(), &name);
        self.refresh();
        result
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

    pub fn path_below(&mut self) -> Option<*mut Self> {
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

    pub fn path_above(&mut self) -> Option<*mut Self> {
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

    pub fn last_child(&mut self) -> Option<*mut Self> {
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

    fn display(&self) -> &str {
        match self {
            Self::File { dispaly, .. } => dispaly,
            Self::Folder { dispaly, .. } => dispaly,
        }
    }

    fn style(&self) -> Style {
        match self {
            Self::File { style, .. } => *style,
            Self::Folder { style, .. } => *style,
        }
    }

    pub fn as_list_item(&self) -> ListItem {
        ListItem::new(self.display()).style(self.style())
    }
}

fn get_path_display(path: &Path) -> String {
    let path_str = &path.display().to_string()[2..];
    let mut buffer = String::new();
    let mut path_split = path_str.split(DIR_SEP).peekable();
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

pub fn new_file_or_folder(mut path: PathBuf, name: &str) -> Result<PathBuf> {
    if let Some(folder) = name.strip_suffix(DIR_SEP) {
        path.push(folder);
        std::fs::create_dir(&path)?;
    } else {
        path.push(name);
        if path.exists() {
            return Err(anyhow!("File already exists! {:?}", path));
        }
        std::fs::write(&path, "")?;
    }
    Ok(path)
}

pub fn order_tree_path(left: &TreePath, right: &TreePath) -> Ordering {
    match (matches!(left, TreePath::Folder { .. }), matches!(right, TreePath::Folder { .. })) {
        (true, true) => Ordering::Equal,
        (false, false) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}
