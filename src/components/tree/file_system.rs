use super::tree_paths::{self, order_tree_path, TreePath};
use crate::utils::get_nested_paths;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
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
            return Ok(());
        }
        Err(anyhow!("No file selected!"))
    }

    pub fn new_file(&mut self, name: String) -> Option<PathBuf> {
        if let Some(parent) = self.get_selected().and_then(|selected| selected.parent()) {
            return parent.new_file(name).ok();
        } else {
            let mut path = PathBuf::from("./");
            path.push(name);
            if !path.exists() {
                std::fs::write(&path, "").ok()?;
                self.force_refresh();
                return Some(path);
            }
        }
        None
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
        std::fs::rename(selected.path(), new_name).ok()?;
        let path_ref = selected.path_mut();
        path_ref.pop();
        path_ref.push(new_name);
        Some(())
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

    fn select(&mut self, ptr: *mut TreePath) {
        if let Some(tree_path) = self.get_selected() {
            tree_path.deselect();
        }
        self.selected = ptr;
        if let Some(tree_path) = self.get_selected() {
            tree_path.select();
        }
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
