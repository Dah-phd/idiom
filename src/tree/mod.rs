mod tree_paths;
use crate::{
    configs::{TreeAction, TreeKeyMap},
    error::{IdiomError, IdiomResult},
    global_state::{GlobalState, WorkspaceEvent},
    lsp::Diagnostic,
    popups::popups_tree::{create_file_popup, rename_file_popup},
    render::state::State,
    utils::{build_file_or_folder, to_relative_path},
};
use crossterm::event::KeyEvent;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;
use tree_paths::TreePath;

const TICK: Duration = Duration::from_millis(500);

pub struct Tree {
    pub key_map: TreeKeyMap,
    state: State,
    selected_path: PathBuf,
    tree: TreePath,
    tree_ptrs: Vec<*mut TreePath>,
    sync_handler: JoinHandle<TreePath>,
    rebuild: bool,
    pub lsp_register: Vec<Arc<Mutex<HashMap<PathBuf, Diagnostic>>>>,
}

impl Tree {
    pub fn new(key_map: TreeKeyMap) -> Self {
        let mut tree = TreePath::default();
        let mut sync_tree = tree.clone();
        let mut tree_ptrs = Vec::new();
        let sync_handler = tokio::spawn(async move {
            tokio::time::sleep(TICK).await;
            sync_tree.sync_base();
            sync_tree
        });
        tree.sync_flat_ptrs(&mut tree_ptrs);
        Self {
            state: State::new(),
            key_map,
            selected_path: PathBuf::from("./"),
            tree,
            tree_ptrs,
            sync_handler,
            rebuild: true,
            lsp_register: Vec::new(),
        }
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        let options = self.tree_ptrs.iter().flat_map(|ptr| unsafe { ptr.as_ref() }.map(|tp| tp.direct_display()));
        self.state.render_list_styled(options, &gs.tree_area, &mut gs.writer);
    }

    #[inline]
    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        if self.rebuild {
            self.rebuild = false;
            self.render(gs);
        };
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        if let Some(action) = self.key_map.map(key) {
            match action {
                TreeAction::Up => self.select_up(),
                TreeAction::Down => self.select_down(),
                TreeAction::Shrink => self.shrink(),
                TreeAction::Expand => {
                    if let Some(path) = self.expand_dir_or_get_path() {
                        gs.workspace.push(WorkspaceEvent::Open(path, 0));
                    }
                }
                TreeAction::Delete => {
                    let _ = self.delete_file();
                }
                TreeAction::NewFile => gs.popup(create_file_popup(self.get_first_selected_folder_display())),
                TreeAction::Rename => {
                    if let Some(tree_path) = self.get_selected() {
                        gs.popup(rename_file_popup(tree_path.path().display().to_string()));
                    }
                }
                TreeAction::IncreaseSize => gs.expand_tree_size(),
                TreeAction::DecreaseSize => gs.shrink_tree_size(),
            }
            return true;
        }
        false
    }

    pub fn expand_dir_or_get_path(&mut self) -> Option<PathBuf> {
        let tree_path = self.get_selected()?;
        if tree_path.path().is_dir() {
            tree_path.expand();
            self.force_sync();
            None
        } else {
            Some(tree_path.path().clone())
        }
    }

    fn shrink(&mut self) {
        if let Some(tree_path) = self.get_selected() {
            tree_path.take_tree();
            self.force_sync();
        }
    }

    pub fn mouse_select(&mut self, idx: usize) -> Option<PathBuf> {
        if self.tree_ptrs.len() >= idx {
            self.state.selected = idx.saturating_sub(1);
            if let Some(selected) = self.get_selected() {
                match selected {
                    TreePath::Folder { tree: Some(..), .. } => {
                        selected.take_tree();
                    }
                    TreePath::Folder { tree: None, .. } => selected.expand(),
                    TreePath::File { path, .. } => {
                        return Some(path.clone());
                    }
                }
                self.selected_path = selected.path().clone();
            };
            self.force_sync();
        }
        None
    }

    fn select_up(&mut self) {
        if self.tree_ptrs.is_empty() {
            return;
        }
        self.state.prev(self.tree_ptrs.len());
        self.unsafe_set_path();
    }

    fn select_down(&mut self) {
        if self.tree_ptrs.is_empty() {
            return;
        }
        self.state.next(self.tree_ptrs.len());
        self.unsafe_set_path();
    }

    pub fn create_file_or_folder(&mut self, name: String) -> IdiomResult<PathBuf> {
        let path = build_file_or_folder(self.selected_path.clone(), &name)?;
        self.force_sync();
        self.select_by_path(&path);
        Ok(path)
    }

    pub fn create_file_or_folder_base(&mut self, name: String) -> IdiomResult<PathBuf> {
        let path = build_file_or_folder(PathBuf::from("./"), &name)?;
        self.force_sync();
        self.select_by_path(&path);
        Ok(path)
    }

    fn delete_file(&mut self) -> IdiomResult<()> {
        if self.selected_path.is_file() {
            std::fs::remove_file(&self.selected_path)?
        } else {
            std::fs::remove_dir_all(&self.selected_path)?
        };
        self.select_up();
        self.force_sync();
        Ok(())
    }

    pub fn rename_path(&mut self, name: String) -> Option<IdiomResult<(PathBuf, PathBuf)>> {
        // not efficient but safe - calls should be rare enough
        let selected = self.get_selected()?;
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
                std::fs::rename(&old_path, &abs_new_path).map(|_| (old_path, abs_new_path))
            })
            .map_err(IdiomError::from);
        if result.is_ok() {
            rel_new_path.push(name);
            selected.update_path(rel_new_path);
            self.force_sync();
        }
        Some(result)
    }

    pub fn search_paths(&self, pattern: &str) -> Vec<PathBuf> {
        self.tree.shallow_copy().search_tree_paths(pattern)
    }

    pub fn shallow_copy_root_tree_path(&self) -> TreePath {
        self.tree.shallow_copy()
    }

    pub fn shallow_copy_selected_tree_path(&self) -> TreePath {
        match self.get_selected() {
            Some(tree_path) => tree_path.shallow_copy(),
            None => self.shallow_copy_root_tree_path(),
        }
    }

    pub fn select_by_path(&mut self, path: &PathBuf) {
        let rel_result = to_relative_path(path);
        let path = rel_result.as_ref().unwrap_or(path);
        if self.tree.expand_contained(path) {
            self.state.selected = 0;
            self.selected_path = path.clone();
            self.force_sync();
        }
    }

    pub fn get_first_selected_folder_display(&self) -> String {
        if let Some(tree_path) = self.get_selected() {
            if tree_path.path().is_dir() {
                return tree_path.path().as_path().display().to_string();
            }
            if let Some(parent) = tree_path.path().parent() {
                return parent.display().to_string();
            }
        }
        "./".to_owned()
    }

    #[inline]
    pub fn get_selected(&self) -> Option<&mut TreePath> {
        unsafe { self.tree_ptrs.get(self.state.selected)?.as_mut() }
    }

    pub async fn finish_sync(&mut self, gs: &mut GlobalState) {
        if self.sync_handler.is_finished() {
            self.rebuild = true;
            let mut tree = self.tree.clone();
            let lsp_register = self.lsp_register.clone();
            let old_handler = std::mem::replace(
                &mut self.sync_handler,
                tokio::spawn(async move {
                    tokio::time::sleep(TICK).await;
                    tree.sync_base();
                    let mut buffer = Vec::new();
                    for lsp in lsp_register.into_iter() {
                        if let Ok(lock) = lsp.try_lock() {
                            for (path, diagnostic) in lock.iter() {
                                buffer.push((path.clone(), diagnostic.errors, diagnostic.warnings));
                            }
                        }
                    }
                    for (path, d_errors, d_warnings) in buffer {
                        tree.map_diagnostics_base(path, d_errors, d_warnings);
                    }
                    tree
                }),
            );
            match old_handler.await {
                Ok(tree) => {
                    self.tree = tree;
                    self.tree.sync_flat_ptrs(&mut self.tree_ptrs);
                    self.fix_select_by_path();
                }
                Err(err) => {
                    gs.error(format!("Tree sync error: {err}"));
                }
            }
        }
        self.tree.sync_flat_ptrs(&mut self.tree_ptrs);
    }

    fn force_sync(&mut self) {
        self.rebuild = true;
        let mut tree = self.tree.clone();
        std::mem::replace(
            &mut self.sync_handler,
            tokio::spawn(async move {
                tree.sync_base();
                tree
            }),
        )
        .abort();
        self.tree.sync_flat_ptrs(&mut self.tree_ptrs);
    }

    fn fix_select_by_path(&mut self) {
        if let Some(selected) = self.get_selected() {
            if &self.selected_path != selected.path() {
                for (idx, tree_path) in self.tree_ptrs.iter_mut().flat_map(|ptr| unsafe { ptr.as_mut() }).enumerate() {
                    if tree_path.path() == &self.selected_path {
                        self.state.selected = idx;
                        break;
                    }
                }
            }
        }
    }

    fn unsafe_set_path(&mut self) {
        self.rebuild = true;
        if let Some(selected) = self.get_selected() {
            self.selected_path = selected.path().clone();
        }
    }

    pub fn get_base_file_names(&self) -> Vec<String> {
        self.tree.tree_file_names()
    }
}
