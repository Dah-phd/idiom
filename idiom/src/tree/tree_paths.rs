use ignore::{gitignore::Gitignore, Match};
use tokio::task::JoinSet;

use crate::{error::IdiomResult, lsp::DiagnosticType, utils::get_nested_paths};
use crossterm::style::{Color, ContentStyle};
use idiom_ui::{
    backend::{Backend, StyleExt},
    layout::Line,
};
use std::{
    cmp::Ordering,
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{watcher::TreeWatcher, PathParser};

const ERR: Color = Color::Red;
const WAR: Color = Color::DarkYellow;

#[derive(Debug, Clone)]
pub enum TreePath {
    Folder { path: PathBuf, tree: Option<Vec<TreePath>>, display: String, diagnostic: DiagnosticType },
    File { path: PathBuf, display: String, diagnostic: DiagnosticType },
}

#[allow(dead_code)]
impl TreePath {
    pub fn from_path(path: PathBuf) -> IdiomResult<Self> {
        if !path.is_dir() {
            return Ok(Self::File { display: get_path_display(&path), path, diagnostic: DiagnosticType::None });
        }
        let mut tree_buffer = get_nested_paths(&path)?
            .filter_map(|p| if is_git_dir(&p) { None } else { Some(p.into()) })
            .collect::<Vec<Self>>();
        tree_buffer.sort_by(order_tree_paths);
        Ok(Self::Folder {
            display: get_path_display(&path),
            path,
            tree: Some(tree_buffer),
            diagnostic: DiagnosticType::None,
        })
    }

    pub fn render(&self, char_offset: usize, line: Line, base_style: ContentStyle, backend: &mut impl Backend) {
        let (display, diagnostic) = match self {
            TreePath::File { display, diagnostic, .. } => (&display[char_offset..], *diagnostic),
            TreePath::Folder { display, diagnostic, tree: Some(..), .. } => {
                (&display[char_offset..display.len() - 2], *diagnostic)
            }
            TreePath::Folder { display, diagnostic, .. } => (&display[char_offset..], *diagnostic),
        };
        match diagnostic {
            DiagnosticType::Err => line.render_styled(display, base_style.with_fg(ERR), backend),
            DiagnosticType::Warn => line.render_styled(display, base_style.with_fg(WAR), backend),
            DiagnosticType::None => line.render_styled(display, base_style, backend),
        };
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

    pub fn expand(&mut self) -> IdiomResult<()> {
        if let Self::Folder { tree, path, .. } = self {
            if tree.is_some() {
                return Ok(());
            }
            let mut buffer = Vec::new();
            for nested_path in get_nested_paths(path)? {
                buffer.push(nested_path.into())
            }
            buffer.sort_by(order_tree_paths);
            tree.replace(buffer);
        }
        Ok(())
    }

    pub fn expand_contained(&mut self, rel_path: &Path, watcher: &mut TreeWatcher) -> IdiomResult<bool> {
        if self.path() == rel_path {
            return Ok(true);
        }

        if !rel_path.starts_with(self.path()) {
            return Ok(false);
        }

        self.expand()?;
        let Some(nested_tree) = self.tree_mut() else {
            return Ok(false);
        };

        for tree_path in nested_tree {
            let result = tree_path.expand_contained(rel_path, watcher);
            if matches!(result, Ok(true)) {
                _ = watcher.watch(self.path());
                return result;
            }
        }

        Ok(false)
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
            Self::Folder { path, display, .. } => Self::Folder {
                path: path.clone(),
                tree: None,
                display: display.clone(),
                diagnostic: DiagnosticType::None,
            },
        }
    }

    /// SYNC with real tree >> should always be possible
    pub fn sync_base(&mut self) {
        if let Self::Folder { path, tree: Some(tree), .. } = self {
            merge_trees(tree, get_nested_paths(path).unwrap().filter(|p| !is_git_dir(p)).collect());
        }
    }

    pub fn sync(&mut self) -> IdiomResult<()> {
        self.reset_diagnostic();
        if let Self::Folder { path, tree: Some(tree), .. } = self {
            merge_trees(tree, get_nested_paths(path)?.collect());
        }
        Ok(())
    }

    // Search utils

    pub fn get_from_inner(&self, idx: usize) -> Option<&TreePath> {
        self.iter().nth(idx + 1)
    }

    pub fn get_mut_from_inner(&mut self, idx: usize) -> Option<&mut TreePath> {
        self.serch_by_idx(idx + 1).into()
    }

    fn serch_by_idx(&mut self, mut idx: usize) -> SerachResult {
        if idx == 0 {
            return SerachResult::Found(self);
        }
        idx -= 1;
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
        let _ = self.expand(); // ignored for now
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
                    if is_git_dir(tree_path.path()) {
                        continue;
                    }
                    tree_path.search_in_files(Arc::clone(&pattern), buffer, gitignore);
                }
            }
            _ => (),
        }
    }

    pub fn search_tree_paths(self, pattern: &str) -> IdiomResult<Vec<PathBuf>> {
        let mut buffer = Vec::new();
        let gitignore = Gitignore::new("./.gitignore").0;
        self.search_in_paths(pattern, &mut buffer, &gitignore)?;
        Ok(buffer)
    }

    pub fn find_by_path_skip_root(&mut self, search_path: &Path, path_parser: PathParser) -> Option<&mut Self> {
        let search_path = path_parser(search_path).ok()?;
        match self {
            Self::Folder { path, tree: Some(inner_tree), .. } if !path.starts_with(&search_path) => {
                for tree_path in inner_tree {
                    if search_path.starts_with(tree_path.path()) {
                        return tree_path.find_by_path(&search_path);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn find_by_path(&mut self, search_path: &Path) -> Option<&mut Self> {
        match self {
            Self::File { path, .. } | Self::Folder { path, .. } if path == search_path => Some(self),
            Self::Folder { tree: Some(inner_tree), .. } => {
                for tree_path in inner_tree {
                    if search_path.starts_with(tree_path.path()) {
                        return tree_path.find_by_path(search_path);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn search_in_paths(
        mut self,
        pattern: &str,
        buffer: &mut Vec<PathBuf>,
        gitignore: &Gitignore,
    ) -> IdiomResult<()> {
        let path = self.path();
        if matches!(gitignore.matched(path, path.is_dir()), Match::Ignore(..)) {
            return Ok(());
        }
        self.expand()?;
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
                            tree_path.collect_all_paths(buffer)?;
                        }
                    }
                } else if let Some(tree) = tree {
                    for tree_path in tree {
                        if is_git_dir(tree_path.path()) {
                            continue;
                        }
                        tree_path.search_in_paths(pattern, buffer, gitignore)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn collect_all_paths(mut self, buffer: &mut Vec<PathBuf>) -> IdiomResult<()> {
        self.expand()?;
        match self {
            Self::File { path, .. } => buffer.push(path),
            Self::Folder { path, tree, .. } => {
                buffer.push(path);
                if let Some(tree) = tree {
                    for tree_path in tree {
                        tree_path.collect_all_paths(buffer)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn search_files_join_set(self, pattern: String) -> JoinSet<Vec<(PathBuf, String, usize)>> {
        let mut buffer = JoinSet::new();
        let gitgnore = Gitignore::new("./.gitignore").0;
        self.search_in_files(pattern.into(), &mut buffer, &gitgnore);
        buffer
    }

    // Diagnostics

    pub fn map_diagnostics_base(&mut self, d_path: &PathBuf, new_diagnostic: DiagnosticType) {
        if let Self::Folder { tree: Some(tree), .. } = self {
            for tree_path in tree {
                tree_path.map_diagnostics(d_path, new_diagnostic);
            }
        }
    }

    fn map_diagnostics(&mut self, d_path: &PathBuf, new_diagnostic: DiagnosticType) -> bool {
        match self {
            Self::Folder { path, tree, diagnostic, .. } => {
                if !d_path.starts_with(path) {
                    return false;
                }
                *diagnostic = new_diagnostic;
                if let Some(tree) = tree {
                    for tree_path in tree.iter_mut() {
                        if tree_path.map_diagnostics(d_path, new_diagnostic) {
                            return true;
                        }
                    }
                }
            }
            Self::File { path, diagnostic, .. } => {
                if path == d_path {
                    *diagnostic = new_diagnostic;
                    return true;
                }
            }
        }
        false
    }

    fn reset_diagnostic(&mut self) {}

    pub fn iter(&self) -> TreeIter {
        TreeIter { holder: vec![self] }
    }
}

impl From<PathBuf> for TreePath {
    fn from(value: PathBuf) -> Self {
        let display = get_path_display(&value);
        if value.is_dir() {
            Self::Folder { path: value, tree: None, display, diagnostic: DiagnosticType::None }
        } else {
            Self::File { path: value, display, diagnostic: DiagnosticType::None }
        }
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

impl<'a> From<SerachResult<'a>> for Option<&'a mut TreePath> {
    fn from(val: SerachResult<'a>) -> Self {
        match val {
            SerachResult::Found(tree_path) => Some(tree_path),
            SerachResult::Remainder(..) => None,
        }
    }
}

impl<'a> From<SerachResult<'a>> for Option<&'a TreePath> {
    fn from(val: SerachResult<'a>) -> Self {
        match val {
            SerachResult::Found(tree_path) => Some(tree_path),
            SerachResult::Remainder(..) => None,
        }
    }
}

fn get_path_display(path: &Path) -> String {
    let path_str = &path.display().to_string();
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
            let _ = tree_path.sync();
            return true;
        }
        false
    });
    tree.sort_by(order_tree_paths)
}

fn is_git_dir(path: &Path) -> bool {
    path.file_name().and_then(|os_str| os_str.to_str()) == Some(".git")
}
