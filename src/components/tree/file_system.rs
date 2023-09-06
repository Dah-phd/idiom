use super::tree_paths::{new_file_or_folder, order_tree_path, TreePath};
use crate::utils::get_nested_paths;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tui::widgets::List;

const TICK: Duration = Duration::from_millis(500);

pub struct FileSystem {
    base_path: PathBuf,
    tree: Vec<TreePath>,
    selected: *mut TreePath,
    select_base_idx: usize,
    ignores: [PathBuf; 1],
    clock: Instant,
}

impl Default for FileSystem {
    fn default() -> Self {
        let ignores: [PathBuf; 1] = [PathBuf::from("./.git")];
        let base_path = PathBuf::from("./");
        let mut tree = get_nested_paths(&base_path)
            .filter(|path| !ignores.contains(path))
            .map(|path| TreePath::with_parent(path, std::ptr::null_mut()))
            .collect::<Vec<_>>();
        tree.sort_by(order_tree_path);
        Self { base_path, tree, selected: std::ptr::null_mut(), clock: Instant::now(), select_base_idx: 0, ignores }
    }
}

impl FileSystem {
    pub fn get_selected(&mut self) -> Option<&mut TreePath> {
        unsafe { self.selected.as_mut() }
    }

    pub fn delete(&mut self) -> Result<()> {
        if let Some(path) = self.get_selected().map(|selected| selected.path()) {
            if path.is_file() {
                std::fs::remove_file(path)?;
            } else {
                std::fs::remove_dir_all(path)?;
            }
            self.select_up();
            self.force_refresh();
            self.reset_select_idx();
            return Ok(());
        }
        Err(anyhow!("No file selected!"))
    }

    pub fn new_file_or_folder(&mut self, name: String) -> Option<PathBuf> {
        if let Some(selected) = self.get_selected() {
            if matches!(selected, TreePath::Folder { .. }) {
                return selected.new_file_or_folder(name).ok();
            };
            selected.refresh();
            if let Some(parent) = selected.parent() {
                return parent.new_file_or_folder(name).ok();
            }
        }
        self.new_file_or_folder_base(name)
    }

    pub fn new_file_or_folder_base(&mut self, name: String) -> Option<PathBuf> {
        let path = PathBuf::from("./");
        let result = new_file_or_folder(path, &name).ok()?;
        self.drop_select();
        self.force_refresh();
        self.select_in_base_path(&result);
        Some(result)
    }

    pub fn open(&mut self) -> Option<PathBuf> {
        match self.get_selected()? {
            TreePath::File { path, .. } => Some(path.clone()),
            folder => {
                folder.expand();
                None
            }
        }
    }

    pub fn rename(&mut self, new_name: &str) -> Option<()> {
        let selected = self.get_selected()?;
        let result = selected.rename(new_name);
        self.force_refresh();
        result
    }

    pub fn close(&mut self) {
        if let Some(TreePath::Folder { tree, .. }) = self.get_selected() {
            *tree = None;
        }
    }

    pub fn as_widget(&mut self) -> List<'_> {
        let should_refresh = self.clock.elapsed() >= TICK;
        let mut buffer = vec![];
        if should_refresh {
            self.force_refresh();
        }
        for path in self.tree.iter_mut() {
            if should_refresh {
                path.refresh();
            }
            buffer.extend(path.as_widgets())
        }
        List::new(buffer)
    }

    pub fn select_down(&mut self) {
        if let Some(selected) = self.get_selected() {
            if let Some(ptr) = selected.path_below() {
                self.select(ptr);
            } else if self.select_base_idx < self.tree.len() - 1 {
                self.select_base_idx += 1;
                let ptr = &mut self.tree[self.select_base_idx] as *mut TreePath;
                self.select(ptr);
            }
        } else if let Some(first) = self.tree.first_mut() {
            let ptr = first as *mut TreePath;
            self.select(ptr);
            self.select_base_idx = 0
        };
    }

    pub fn select_up(&mut self) {
        if let Some(selected) = self.get_selected() {
            if let Some(ptr) = selected.path_above() {
                self.select(ptr);
            } else if self.select_base_idx > 0 {
                self.select_base_idx -= 1;
                let base_path = &mut self.tree[self.select_base_idx];
                let ptr = if let Some(ptr) = base_path.last_child() { ptr } else { base_path };
                self.select(ptr);
            }
        } else if let Some(last) = self.tree.last_mut() {
            let ptr: *mut TreePath = last;
            self.select(ptr);
            self.select_base_idx = self.tree.len() - 1;
        };
    }

    fn reset_select_idx(&mut self) {
        if let Some(selected) = self.get_selected() {
            let path = selected.path().clone();
            for (idx, tree_path) in self.tree.iter().enumerate() {
                if tree_path.is_parent(&path) {
                    self.select_base_idx = idx;
                    return;
                }
            }
        }
    }

    fn select_in_base_path(&mut self, path: &PathBuf) {
        for tree_path in self.tree.iter_mut() {
            if path == tree_path.path() {
                tree_path.select();
                self.selected = tree_path;
                self.reset_select_idx();
                return;
            }
        }
    }

    fn select(&mut self, ptr: *mut TreePath) {
        if let Some(tree_path) = self.get_selected() {
            tree_path.deselect();
        }
        self.selected = ptr;
        if let Some(tree_path) = self.get_selected() {
            tree_path.select();
        }
    }

    fn drop_select(&mut self) {
        if let Some(tree_path) = self.get_selected() {
            tree_path.deselect();
        }
        self.selected = std::ptr::null_mut();
    }

    fn force_refresh(&mut self) {
        self.refresh();
        self.clock = Instant::now()
    }

    fn refresh(&mut self) {
        let updated_tree = get_nested_paths(&self.base_path).filter(|p| !self.ignores.contains(p)).collect::<Vec<_>>();
        for path in updated_tree.iter() {
            if !self.tree.iter().any(|tp| tp.path() == path) {
                self.tree.push(TreePath::with_parent(path.clone(), std::ptr::null_mut()))
            }
        }
        self.tree.retain_mut(|el| {
            if updated_tree.contains(el.path()) {
                el.refresh();
                return true;
            }
            false
        });
        self.tree.sort_by(order_tree_path)
    }
}
