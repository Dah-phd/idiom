use super::tree_paths::{order_tree_path, TreePath};
use crate::utils::get_nested_paths;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tui::widgets::ListItem;

const TICK: Duration = Duration::from_millis(500);

pub struct FileSystem {
    tree: Vec<TreePath>,
    selected: *mut TreePath,
    select_base_idx: usize,
    clock: Instant,
}

impl Default for FileSystem {
    fn default() -> Self {
        let ignores: [PathBuf; 1] = [PathBuf::from("./.git")];
        let path = PathBuf::from("./");
        let mut tree = get_nested_paths(&path)
            .filter(|path| !ignores.contains(path))
            .map(|path| TreePath::with_parent(path, std::ptr::null_mut()))
            .collect::<Vec<_>>();
        tree.sort_by(order_tree_path);
        Self { tree, selected: std::ptr::null_mut(), clock: Instant::now(), select_base_idx: 0 }
    }
}

impl FileSystem {
    pub fn get_selected(&mut self) -> Option<&mut TreePath> {
        unsafe { self.selected.as_mut() }
    }

    pub fn new_file(&mut self, name: String) -> Option<PathBuf> {
        if let Some(selected) = self.get_selected() {
            return if let Some(parent) = selected.parent() { parent } else { selected }.new_file(name).ok();
        } else {
            let mut path = PathBuf::from("./");
            path.push(name);
            if !path.exists() {
                std::fs::write(&path, "").ok()?;
                let mut tree_path = TreePath::with_parent(path.clone(), std::ptr::null_mut());
                self.selected = &mut tree_path;
                self.tree.push(tree_path);
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

    fn rename(&mut self, new_name: &str) -> Option<()> {
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

    pub fn as_widgets(&mut self) -> Vec<ListItem<'_>> {
        let should_refresh = self.clock.elapsed() >= TICK;
        let mut buffer = vec![];
        let selected_path = self.get_selected().map(|selected| selected.path()).cloned();
        for path in self.tree.iter_mut() {
            if should_refresh {
                path.refresh();
            }
            buffer.extend(path.as_widgets(&selected_path))
        }
        if should_refresh {
            self.clock = Instant::now();
        }
        buffer
    }

    pub fn select_next(&mut self) {
        if let Some(selected) = self.get_selected() {
            if let Some(ptr) = selected.next() {
                self.selected = ptr;
            } else if self.select_base_idx < self.tree.len() - 1 {
                self.select_base_idx += 1;
                self.selected = &mut self.tree[self.select_base_idx];
            }
        } else if let Some(first) = self.tree.first_mut() {
            self.selected = first;
            self.select_base_idx = 0
        };
    }

    pub fn select_prev(&mut self) {
        if let Some(selected) = self.get_selected() {
            if let Some(ptr) = selected.prev() {
                self.selected = ptr;
            } else if self.select_base_idx > 0 {
                self.select_base_idx -= 1;
                self.selected = &mut self.tree[self.select_base_idx];
            }
        } else if let Some(last) = self.tree.last_mut() {
            self.selected = last;
            self.select_base_idx = self.tree.len() - 1;
        };
    }
}
