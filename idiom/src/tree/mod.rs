mod file_clipboard;
mod tree_paths;
mod watcher;
use crate::{
    configs::{TreeAction, TreeKeyMap},
    error::{IdiomError, IdiomResult},
    global_state::{GlobalState, IdiomEvent},
    lsp::{DiagnosticType, TreeDiagnostics},
    popups::popups_tree::{create_file_popup, create_root_file_popup, rename_file_popup},
    utils::{build_file_or_folder, to_canon_path, to_relative_path},
};
use crossterm::event::KeyEvent;
use file_clipboard::FileClipboard;
use idiom_ui::{
    backend::{Backend, StyleExt},
    state::State,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    path::{Path, PathBuf},
};
pub use tree_paths::TreePath;
use watcher::TreeWatcher;

type PathParser = fn(&Path) -> IdiomResult<PathBuf>;

pub struct Tree {
    pub key_map: TreeKeyMap,
    pub watcher: TreeWatcher,
    tree_clipboard: FileClipboard,
    state: State,
    diagnostics_state: HashMap<PathBuf, DiagnosticType>,
    selected_path: PathBuf,
    tree: TreePath,
    display_offset: usize,
    path_parser: PathParser,
    rebuild: bool,
}

impl Tree {
    pub fn new(key_map: TreeKeyMap, gs: &mut GlobalState) -> Self {
        match PathBuf::from("./").canonicalize() {
            Ok(selected_path) => {
                let path_str = selected_path.display().to_string();
                let display_offset = path_str.split(std::path::MAIN_SEPARATOR).count() * 2;
                let tree = TreePath::from_path(selected_path.clone()).unwrap();
                Self {
                    watcher: TreeWatcher::root(&selected_path),
                    state: State::new(),
                    tree_clipboard: FileClipboard::default(),
                    key_map,
                    display_offset,
                    path_parser: to_canon_path,
                    selected_path,
                    tree,
                    rebuild: true,
                    diagnostics_state: HashMap::new(),
                }
            }
            Err(err) => {
                gs.error(err.to_string());
                let selected_path = PathBuf::from("./");
                let tree = TreePath::from_path(selected_path.clone()).unwrap();
                Self {
                    watcher: TreeWatcher::root(&selected_path),
                    state: State::new(),
                    tree_clipboard: FileClipboard::default(),
                    key_map,
                    display_offset: 2,
                    path_parser: to_relative_path,
                    selected_path,
                    tree,
                    rebuild: true,
                    diagnostics_state: HashMap::new(),
                }
            }
        }
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        let mut iter = self.tree.iter();
        iter.next();
        let mut lines = gs.tree_area.into_iter();
        let base_style = gs.theme.accent_style;
        let select_base_style = self.state.highlight.with_bg(gs.theme.accent_background);
        for (idx, tree_path) in iter.enumerate().skip(self.state.at_line) {
            let Some(mut line) = lines.next() else { return };
            let style = match idx == self.state.selected {
                true => select_base_style,
                false => base_style,
            };
            if let Some(mark) = self.tree_clipboard.get_mark(tree_path.path()) {
                if mark.len() + 10 < line.width {
                    line.width -= mark.len();
                    gs.backend.print_styled_at(line.row, line.col + line.width as u16, mark, style);
                }
            }
            tree_path.render(self.display_offset, line, style, &mut gs.backend);
        }
        for line in lines {
            line.fill_styled(' ', base_style, &mut gs.backend);
        }
    }

    #[inline]
    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        if self.rebuild {
            self.rebuild = false;
            let accent = Some(gs.theme.accent_background);
            gs.backend().set_bg(accent);
            self.render(gs);
            gs.backend().reset_style();
        };
    }

    #[inline(always)]
    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        self.key_map.map(key).map(|action| self.map_action(action, gs)).unwrap_or_default()
    }

    pub fn map_action(&mut self, action: TreeAction, gs: &mut GlobalState) -> bool {
        match action {
            TreeAction::Up => self.select_up(gs),
            TreeAction::Down => self.select_down(gs),
            TreeAction::Shrink => self.shrink(gs),
            TreeAction::Expand => {
                if let Some(path) = self.expand_dir_or_get_path(gs) {
                    gs.event.push(IdiomEvent::OpenAtLine(path, 0));
                }
            }
            TreeAction::Delete => {
                let _ = self.delete_file(gs);
            }
            TreeAction::NewFile => {
                let root = self.tree.path().to_owned();
                match self.tree.get_mut_from_inner(self.state.selected) {
                    // root cannot be file
                    Some(TreePath::File { path, .. }) => match path.parent() {
                        Some(parent) if parent != root => gs.event.push(create_file_popup(parent.to_owned()).into()),
                        _ => gs.event.push(create_root_file_popup().into()),
                    },
                    // in case folder is not expanded create in parant
                    Some(TreePath::Folder { path, tree: None, .. }) => {
                        // should be unreachable
                        if &root == path {
                            gs.event.push(create_root_file_popup().into());
                        } else {
                            match path.parent() {
                                Some(parent) if parent != root => {
                                    gs.event.push(create_file_popup(parent.to_owned()).into())
                                }
                                _ => gs.event.push(create_root_file_popup().into()),
                            }
                        }
                    }
                    // create in selected dir (root check)
                    Some(TreePath::Folder { path, tree: Some(..), .. }) => match path == &root {
                        true => gs.event.push(create_root_file_popup().into()),
                        false => gs.event.push(create_file_popup(path.to_owned()).into()),
                    },
                    // nothing is selected
                    None => gs.event.push(create_root_file_popup().into()),
                };
            }
            TreeAction::CopyPath => match self.selected_path.canonicalize() {
                Ok(path) => {
                    gs.success("Copied path ...");
                    gs.clipboard.push(path.display().to_string());
                }
                Err(error) => {
                    gs.error(format!("COPIED AS IS (Path resolution error): {error}"));
                }
            },
            TreeAction::CopyPathRelative => match to_relative_path(&self.selected_path) {
                Ok(path) => {
                    gs.success("Copied relative path ...");
                    gs.clipboard.push(path.display().to_string());
                }
                Err(error) => {
                    gs.error(format!("COPIED AS IS (Path resolution error): {error}"));
                }
            },
            TreeAction::CopyFile => {
                if self.tree.path() != &self.selected_path {
                    self.tree_clipboard.force_copy(self.selected_path.to_owned());
                    self.rebuild = true;
                }
            }
            TreeAction::MarkCopyFile => {
                if self.tree.path() != &self.selected_path {
                    self.tree_clipboard.copy(self.selected_path.to_owned());
                    self.rebuild = true;
                }
            }
            TreeAction::CutFile => {
                if self.tree.path() != &self.selected_path {
                    self.tree_clipboard.force_cut(self.selected_path.to_owned());
                    self.rebuild = true;
                }
            }
            TreeAction::MarkCutFile => {
                if self.tree.path() != &self.selected_path {
                    self.tree_clipboard.cut(self.selected_path.to_owned());
                    self.rebuild = true;
                }
            }
            TreeAction::Paste => match self.tree.get_mut_from_inner(self.state.selected) {
                Some(TreePath::Folder { path, tree: Some(..), .. }) => {
                    self.tree_clipboard.paste(path, gs);
                }
                Some(TreePath::Folder { path, tree: None, .. }) | Some(TreePath::File { path, .. }) => {
                    match path.parent() {
                        Some(parent) => self.tree_clipboard.paste(parent, gs),
                        None => gs.error(IdiomError::io_parent_not_found(path)),
                    }
                }
                None => {
                    gs.error("Unable to find selected path ... dropping clipboard and rebasing");
                    self.error_reset();
                }
            },
            TreeAction::Rename => {
                gs.event.push(rename_file_popup(self.selected_path.display().to_string()).into());
            }
            TreeAction::IncreaseSize => gs.expand_tree_size(),
            TreeAction::DecreaseSize => gs.shrink_tree_size(),
        }
        true
    }

    pub fn expand_dir_or_get_path(&mut self, gs: &mut GlobalState) -> Option<PathBuf> {
        let tree_path = self.tree.get_mut_from_inner(self.state.selected)?;
        let path = tree_path.path();
        if path.is_dir() {
            if let Err(err) = self.watcher.watch(path) {
                gs.error(err.to_string());
            };
            match tree_path.expand() {
                Ok(..) => {
                    for (d_path, new_diagnostic) in self.diagnostics_state.iter() {
                        tree_path.map_diagnostics_base(d_path, *new_diagnostic);
                    }
                }
                Err(error) => {
                    gs.error(error);
                }
            };
            self.rebuild = true;
            None
        } else {
            Some(tree_path.path().clone())
        }
    }

    fn shrink(&mut self, gs: &mut GlobalState) {
        if let Some(TreePath::Folder { path, tree, .. }) = self.tree.get_mut_from_inner(self.state.selected) {
            if tree.is_none() {
                return;
            }
            if let Err(err) = self.watcher.stop_watch(path) {
                gs.error(err.to_string());
            };
            *tree = None;
            self.rebuild = true;
        }
    }

    pub fn mouse_select(&mut self, idx: usize, gs: &mut GlobalState) -> Option<PathBuf> {
        if self.tree.len() > idx {
            self.state.selected = idx.saturating_sub(1) + self.state.at_line;
            if let Some(selected) = self.tree.get_mut_from_inner(self.state.selected) {
                match selected {
                    TreePath::Folder { tree: Some(..), .. } => {
                        selected.take_tree();
                    }
                    TreePath::Folder { tree: None, .. } => match selected.expand() {
                        Ok(..) => {
                            for (d_path, new_diagnostic) in self.diagnostics_state.iter() {
                                selected.map_diagnostics_base(d_path, *new_diagnostic);
                            }
                        }
                        Err(error) => gs.error(error),
                    },
                    TreePath::File { path, .. } => {
                        self.selected_path = path.clone();
                        self.rebuild = true;
                        return Some(path.clone());
                    }
                }
                self.selected_path = selected.path().to_owned();
            };
            self.rebuild = true;
        }
        None
    }

    pub fn mouse_menu_setup_select(&mut self, idx: usize) -> bool {
        if self.tree.len() > idx {
            self.state.selected = idx.saturating_sub(1) + self.state.at_line;
            if let Some(selected) = self.tree.get_mut_from_inner(self.state.selected) {
                self.rebuild = true;
                self.selected_path = selected.path().to_owned();
                return true;
            };
        }
        false
    }

    pub fn select_up(&mut self, gs: &mut GlobalState) {
        let tree_len = self.tree.len() - 1;
        if tree_len == 0 {
            return;
        }
        self.state.prev(tree_len);
        self.state.update_at_line(gs.tree_area.height as usize);
        self.unsafe_set_path();
    }

    pub fn select_down(&mut self, gs: &mut GlobalState) {
        let tree_len = self.tree.len() - 1;
        if tree_len == 0 {
            return;
        }
        self.state.next(tree_len);
        self.state.update_at_line(gs.tree_area.height as usize);
        self.unsafe_set_path();
    }

    pub fn push_diagnostics(&mut self, new: TreeDiagnostics) {
        self.rebuild = true;
        for (path, new_diagnostic) in new {
            if let Ok(d_path) = (self.path_parser)(&path) {
                self.tree.map_diagnostics_base(&d_path, new_diagnostic);
                if matches!(new_diagnostic, DiagnosticType::None) {
                    self.diagnostics_state.remove(&d_path);
                    continue;
                }
                match self.diagnostics_state.entry(d_path) {
                    Entry::Occupied(mut entry) => {
                        entry.insert(new_diagnostic);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(new_diagnostic);
                    }
                }
            }
        }
    }

    fn rebuild_diagnostics(&mut self) {
        for (d_path, new_diagnostic) in self.diagnostics_state.iter() {
            self.tree.map_diagnostics_base(d_path, *new_diagnostic);
        }
    }

    pub fn create_file_or_folder(&mut self, name: String) -> IdiomResult<PathBuf> {
        let path = match self.tree.get_mut_from_inner(self.state.selected) {
            Some(TreePath::Folder { path, tree: Some(..), .. }) | Some(TreePath::File { path, .. }) => {
                build_file_or_folder(path.to_owned(), &name)?
            }
            Some(TreePath::Folder { path, tree: None, .. }) => match path.parent() {
                Some(parent) => build_file_or_folder(parent.to_owned(), &name)?,
                None => return Err(IdiomError::io_parent_not_found(path)),
            },
            // build file where wanted
            None => build_file_or_folder(self.selected_path.clone(), &name)?,
        };
        Ok(path)
    }

    pub fn create_file_or_folder_base(&mut self, name: String) -> IdiomResult<PathBuf> {
        let path = build_file_or_folder(self.tree.path().to_owned(), &name)?;
        Ok(path)
    }

    fn delete_file(&mut self, gs: &mut GlobalState) -> IdiomResult<()> {
        if self.selected_path.is_file() {
            std::fs::remove_file(&self.selected_path)?
        } else {
            std::fs::remove_dir_all(&self.selected_path)?
        };
        self.select_up(gs);
        self.rebuild = true;
        Ok(())
    }

    pub fn rename_path(&mut self, name: String) -> Option<IdiomResult<(PathBuf, PathBuf)>> {
        // not efficient but safe - calls should be rare enough
        let selected = self.tree.get_mut_from_inner(self.state.selected)?;
        let mut rel_new_path = selected.path().clone();
        if !rel_new_path.pop() {
            return None;
        };
        let result = selected
            .path()
            .canonicalize()
            .and_then(|old_path| {
                let mut abs_new_path = old_path.clone();
                abs_new_path.pop();
                abs_new_path.push(&name);
                if abs_new_path.exists() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "Unable to rename to already existing path!",
                    ));
                }
                std::fs::rename(&old_path, &abs_new_path).map(|_| (old_path, abs_new_path))
            })
            .map_err(IdiomError::from);
        if result.is_ok() {
            rel_new_path.push(name);
            selected.update_path(rel_new_path);
            self.rebuild = true;
        }
        Some(result)
    }

    pub fn search_paths(&self, pattern: &str) -> Vec<PathBuf> {
        self.tree.shallow_copy().search_tree_paths(pattern).unwrap()
    }

    pub fn shallow_copy_root_tree_path(&self) -> TreePath {
        self.tree.shallow_copy()
    }

    pub fn shallow_copy_selected_tree_path(&self) -> TreePath {
        match self.tree.get_from_inner(self.state.selected) {
            Some(tree_path) => tree_path.shallow_copy(),
            None => self.shallow_copy_root_tree_path(),
        }
    }

    pub fn select_by_path(&mut self, path: &PathBuf) -> IdiomResult<()> {
        let rel_result = (self.path_parser)(path);
        let path = rel_result.as_ref().unwrap_or(path);

        if !path.starts_with(self.tree.path()) {
            return Ok(());
        }

        match self.tree.expand_contained(path, &mut self.watcher) {
            Ok(true) => {
                self.selected_path.clone_from(path);
                self.state.selected = self.tree.iter().skip(1).position(|tp| tp.path() == path).unwrap_or_default();
                self.rebuild_diagnostics();
                self.rebuild = true;
                Ok(())
            }
            Ok(false) => {
                self.state.reset();
                Err(IdiomError::io_not_found("Unable to select file! Setting first as selected"))
            }
            Err(err) => {
                self.error_reset();
                Err(err)
            }
        }
    }

    pub fn get_base_file_names(&self) -> Vec<String> {
        self.tree.tree_file_names()
    }

    pub fn sync(&mut self, gs: &mut GlobalState) {
        self.rebuild = self.watcher.poll(&mut self.tree, self.path_parser, gs);
        if !self.rebuild {
            return;
        }
        for (idx, tree_path) in self.tree.iter().skip(1).enumerate() {
            if tree_path.path() == &self.selected_path {
                self.state.selected = idx;
                return;
            }
        }
        gs.error("IO NotFound: Unable to find selected path .. setting to first path");
        self.error_reset();
    }

    #[inline]
    fn error_reset(&mut self) {
        self.tree_clipboard.clear();
        self.tree.sync_base();
        self.unsafe_set_path();
    }

    /// sets path from idx without checking if it is correct
    fn unsafe_set_path(&mut self) {
        self.rebuild = true;
        if let Some(selected) = self.tree.get_mut_from_inner(self.state.selected) {
            self.selected_path = selected.path().to_owned();
        }
    }
}

#[cfg(test)]
pub mod tests;
