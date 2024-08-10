use ignore::gitignore::Gitignore;
use ignore::Match;
use tokio::task::JoinSet;

use crate::{
    render::backend::{color, Color, Style},
    utils::{get_nested_paths, to_relative_path},
};
use std::{
    cmp::Ordering,
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

const GIT: &str = "./.git";
const ERR: Color = color::red();
const WAR: Color = color::dark_yellow();

#[derive(Debug, Clone)]
pub enum TreePath {
    Folder { path: PathBuf, tree: Option<Vec<TreePath>>, display: String, errors: usize, warnings: usize },
    File { path: PathBuf, display: String, errors: usize, warnings: usize },
}

#[allow(dead_code)]
impl TreePath {
    pub fn clean_from(path: &str) -> Self {
        let path = PathBuf::from(path);
        if !path.is_dir() {
            return Self::File { display: get_path_display(&path), path, errors: 0, warnings: 0 };
        }
        let mut tree_buffer = get_nested_paths(&path)
            .filter_map(|p| if p.starts_with(GIT) { None } else { Some(p.into()) })
            .collect::<Vec<Self>>();
        tree_buffer.sort_by(order_tree_paths);
        Self::Folder { display: get_path_display(&path), path, tree: Some(tree_buffer), errors: 0, warnings: 0 }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Folder { tree: Some(inner), .. } => 1 + inner.iter().map(Self::len).sum::<usize>(),
            _ => 1,
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Folder { path, .. } => path,
            Self::File { path, .. } => path,
        }
    }

    pub fn file_name(&self) -> Option<String> {
        self.path().file_name()?.to_str().map(|s| s.to_string())
    }

    pub fn tree_file_names(&self) -> Vec<String> {
        match self {
            Self::File { .. } => self.file_name().into_iter().collect(),
            Self::Folder { tree, .. } => {
                tree.as_ref().map(|paths| paths.iter().flat_map(|p| p.file_name()).collect()).unwrap_or_default()
            }
        }
    }

    fn tree_mut(&mut self) -> Option<&mut Vec<TreePath>> {
        if let Self::Folder { tree: Some(tree), .. } = self {
            return Some(tree);
        }
        None
    }

    pub fn take_tree(&mut self) -> Option<Vec<Self>> {
        if let Self::Folder { tree, .. } = self {
            return tree.take();
        }
        None
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
            buffer.sort_by(order_tree_paths);
            tree.replace(buffer);
        }
    }

    pub fn expand_contained(&mut self, rel_path: &PathBuf) -> bool {
        if self.path() == rel_path {
            return true;
        }
        if rel_path.starts_with(self.path()) {
            let should_shrink = self.tree_mut().is_none();
            self.expand();
            if let Some(nested_tree) = self.tree_mut() {
                for tree_path in nested_tree {
                    if tree_path.expand_contained(rel_path) {
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

    pub fn update_path(&mut self, new_path: PathBuf) {
        match self {
            Self::File { path, display, .. } => {
                *display = get_path_display(&new_path);
                *path = new_path;
            }
            Self::Folder { path, display, .. } => {
                *display = get_path_display(&new_path);
                *path = new_path;
            }
        }
    }

    pub fn shallow_copy(&self) -> Self {
        match self {
            Self::File { .. } => self.clone(),
            Self::Folder { path, display, .. } => {
                Self::Folder { path: path.clone(), tree: None, display: display.clone(), errors: 0, warnings: 0 }
            }
        }
    }

    /// SYNC with real tree

    pub fn sync_base(&mut self) {
        if let Self::Folder { path, tree: Some(tree), .. } = self {
            merge_trees(tree, get_nested_paths(path).filter(|p| !p.starts_with(GIT)).collect());
        }
    }

    fn sync(&mut self) {
        self.reset_diagnostic();
        if let Self::Folder { path, tree: Some(tree), .. } = self {
            merge_trees(tree, get_nested_paths(path).collect());
        }
    }

    /// Search utils

    pub fn get_from_inner(&self, idx: usize) -> Option<&TreePath> {
        self.iter().nth(idx)
    }

    pub fn get_mut_from_inner(&mut self, idx: usize) -> Option<&mut TreePath> {
        self.serch_by_idx(idx).into()
    }

    fn serch_by_idx(&mut self, mut idx: usize) -> SerachResult {
        idx -= 1;
        if idx == 0 {
            return SerachResult::Found(self);
        }
        if let TreePath::Folder { tree: Some(inner_tree), .. } = self {
            for tree_path in inner_tree.iter_mut() {
                match tree_path.serch_by_idx(idx) {
                    SerachResult::Found(tp) => return SerachResult::Found(tp),
                    SerachResult::Remainder(new_idx) => idx = new_idx,
                }
            }
        }
        SerachResult::Remainder(idx)
    }

    pub fn search_in_files(
        mut self,
        pattern: Arc<str>,
        buffer: &mut JoinSet<Vec<(PathBuf, String, usize)>>,
        gitignore: &Gitignore,
    ) {
        let path = self.path();
        if matches!(gitignore.matched(path, path.is_dir()), Match::Ignore(..)) {
            return;
        };
        self.expand();
        match self {
            Self::File { path, .. } => {
                buffer.spawn(async move {
                    let maybe_content = std::fs::read_to_string(&path);
                    let mut buffer = Vec::new();
                    if let Ok(content) = maybe_content {
                        for (idx, line) in content.lines().enumerate() {
                            if line.contains(&*pattern) {
                                buffer.push((path.clone(), line.trim_start().to_owned(), idx))
                            }
                        }
                    }
                    buffer
                });
            }
            Self::Folder { tree: Some(tree), .. } => {
                for tree_path in tree {
                    if tree_path.path().starts_with(GIT) {
                        continue;
                    }
                    tree_path.search_in_files(Arc::clone(&pattern), buffer, gitignore);
                }
            }
            _ => (),
        }
    }

    pub fn search_tree_paths(self, pattern: &str) -> Vec<PathBuf> {
        let mut buffer = Vec::new();
        let gitignore = Gitignore::new("./.gitignore").0;
        self.search_in_paths(pattern, &mut buffer, &gitignore);
        buffer
    }

    pub fn search_in_paths(mut self, pattern: &str, buffer: &mut Vec<PathBuf>, gitignore: &Gitignore) {
        let path = self.path();
        if matches!(gitignore.matched(path, path.is_dir()), Match::Ignore(..)) {
            return;
        }
        self.expand();
        match self {
            Self::File { path, display, .. } => {
                if display.contains(pattern) {
                    buffer.push(path);
                }
            }
            Self::Folder { path, tree, display, .. } => {
                if display.contains(pattern) {
                    buffer.push(path);
                    if let Some(tree) = tree {
                        for tree_path in tree {
                            tree_path.collect_all_paths(buffer);
                        }
                    }
                } else if let Some(tree) = tree {
                    for tree_path in tree {
                        if tree_path.path().starts_with(GIT) {
                            continue;
                        }
                        tree_path.search_in_paths(pattern, buffer, gitignore);
                    }
                }
            }
        }
    }

    fn collect_all_paths(mut self, buffer: &mut Vec<PathBuf>) {
        self.expand();
        match self {
            Self::File { path, .. } => buffer.push(path),
            Self::Folder { path, tree, .. } => {
                buffer.push(path);
                if let Some(tree) = tree {
                    for tree_path in tree {
                        tree_path.collect_all_paths(buffer);
                    }
                }
            }
        }
    }

    /// Diagnostics

    pub fn map_diagnostics_base(&mut self, path: PathBuf, d_errors: usize, d_warnings: usize) {
        if d_errors == 0 && d_warnings == 0 {
            return;
        }
        if let Ok(d_path) = to_relative_path(&path) {
            if let Self::Folder { tree: Some(tree), .. } = self {
                for tree_path in tree {
                    tree_path.map_diagnostics(&d_path, d_errors, d_warnings);
                }
            }
        };
    }

    fn map_diagnostics(&mut self, d_path: &PathBuf, d_errors: usize, d_warnings: usize) -> bool {
        match self {
            Self::Folder { path, tree, errors, warnings, .. } => {
                if !d_path.starts_with(path) {
                    return false;
                }
                if let Some(tree) = tree {
                    for tree_path in tree.iter_mut() {
                        if tree_path.map_diagnostics(d_path, d_errors, d_warnings) {
                            return true;
                        }
                    }
                }
                *errors = d_errors;
                *warnings = d_warnings;
            }
            Self::File { path, errors, warnings, .. } => {
                if path == d_path {
                    *errors = d_errors;
                    *warnings = d_warnings;
                    return true;
                }
            }
        }
        false
    }

    fn reset_diagnostic(&mut self) {
        match self {
            Self::Folder { errors, warnings, .. } => {
                *errors = 0;
                *warnings = 0;
            }
            Self::File { errors, warnings, .. } => {
                *errors = 0;
                *warnings = 0;
            }
        }
    }

    pub fn iter(&self) -> TreeIter {
        TreeIter { holder: vec![self] }
    }
}

impl From<PathBuf> for TreePath {
    fn from(value: PathBuf) -> Self {
        let display = get_path_display(&value);
        if value.is_dir() {
            Self::Folder { path: value, tree: None, display, errors: 0, warnings: 0 }
        } else {
            Self::File { path: value, display, errors: 0, warnings: 0 }
        }
    }
}

impl Default for TreePath {
    fn default() -> Self {
        Self::clean_from("./")
    }
}

pub struct TreeIter<'a> {
    holder: Vec<&'a TreePath>,
}

impl<'a> Iterator for TreeIter<'a> {
    type Item = &'a TreePath;
    fn next(&mut self) -> Option<Self::Item> {
        self.holder.pop().inspect(|tree_path| {
            if let TreePath::Folder { tree: Some(inner_tree), .. } = tree_path {
                self.holder.extend(inner_tree.iter().rev());
            }
        })
    }
}

enum SerachResult<'a> {
    Found(&'a mut TreePath),
    Remainder(usize),
}

impl<'a> Into<Option<&'a mut TreePath>> for SerachResult<'a> {
    fn into(self) -> Option<&'a mut TreePath> {
        match self {
            Self::Found(tree_path) => Some(tree_path),
            Self::Remainder(..) => None,
        }
    }
}

impl<'a> Into<Option<&'a TreePath>> for SerachResult<'a> {
    fn into(self) -> Option<&'a TreePath> {
        match self {
            Self::Found(tree_path) => Some(tree_path),
            Self::Remainder(..) => None,
        }
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

fn order_tree_paths(left: &TreePath, right: &TreePath) -> Ordering {
    match (left, right) {
        (TreePath::Folder { .. }, TreePath::File { .. }) => Ordering::Less,
        (TreePath::File { .. }, TreePath::Folder { .. }) => Ordering::Greater,
        _ => left.path().cmp(right.path()),
    }
}

fn merge_trees(tree: &mut Vec<TreePath>, new_tree_set: HashSet<PathBuf>) {
    for path in new_tree_set.iter() {
        if !tree.iter().any(|tree_element| tree_element.path() == path) {
            tree.push(path.clone().into())
        }
    }
    tree.retain_mut(|tree_path| {
        if new_tree_set.contains(tree_path.path()) {
            tree_path.sync();
            return true;
        }
        false
    });
    tree.sort_by(order_tree_paths)
}
