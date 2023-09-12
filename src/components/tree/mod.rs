mod tree_paths;
use crate::configs::GeneralAction;
use crate::utils::build_file_or_folder;
use anyhow::Result;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::task::JoinSet;
use tree_paths::TreePath;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

const TICK: Duration = Duration::from_millis(500);

#[derive(Clone)]
pub struct Tree {
    active: bool,
    state: ListState,
    selected_path: PathBuf,
    tree: TreePath,
    tree_ptrs: Vec<*mut TreePath>,
    clock: Instant,
    ignore_base_paths: Vec<PathBuf>,
    pub on_open_tabs: bool,
}

impl Tree {
    pub fn new(active: bool) -> Self {
        let ignore_base_paths = vec![PathBuf::from("./.git")];
        let mut tree = TreePath::default();
        let mut tree_ptrs = Vec::new();
        tree.sync_flat_ptrs(&mut tree_ptrs, &ignore_base_paths);
        Self {
            active,
            state: ListState::default(),
            selected_path: PathBuf::from("./"),
            tree,
            tree_ptrs,
            clock: Instant::now(),
            ignore_base_paths,
            on_open_tabs: false,
        }
    }

    pub fn render_with_remainder(&mut self, frame: &mut Frame<impl Backend>, screen: Rect) -> Rect {
        if !self.active {
            return screen;
        }
        let areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(15), Constraint::Min(2)])
            .split(screen);

        self.sync();

        let mut state = self.state.clone();

        let tree = List::new(self.get_list_items())
            .block(Block::default().borders(Borders::ALL).title("Explorer"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(tree, areas[0], &mut state);
        areas[1]
    }

    pub fn map(&mut self, action: &GeneralAction) -> bool {
        match action {
            GeneralAction::Up => self.select_up(),
            GeneralAction::Down => self.select_down(),
            GeneralAction::Shrink => self.shrink(),
            GeneralAction::DeleteFile => {
                let _ = self.delete_file();
            }
            _ => return false,
        }
        true
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

    fn select_up(&mut self) {
        if self.tree_ptrs.is_empty() {
            return;
        }
        if let Some(idx) = self.state.selected() {
            if idx == 0 {
                return;
            }
            self.unsafe_select(idx - 1);
        } else {
            self.unsafe_select(self.tree_ptrs.len() - 1);
        }
    }

    fn select_down(&mut self) {
        if self.tree_ptrs.is_empty() {
            return;
        }
        if let Some(idx) = self.state.selected() {
            let new_idx = idx + 1;
            if self.tree_ptrs.len() == new_idx {
                return;
            }
            self.unsafe_select(new_idx);
        } else {
            self.unsafe_select(0);
        }
    }

    pub fn create_file_or_folder(&mut self, name: String) -> Result<PathBuf> {
        let path = build_file_or_folder(self.selected_path.clone(), &name)?;
        self.force_sync();
        self.select_by_path(&path);
        Ok(path)
    }

    pub fn create_file_or_folder_base(&mut self, name: String) -> Result<PathBuf> {
        let path = build_file_or_folder(PathBuf::from("./"), &name)?;
        self.force_sync();
        self.select_by_path(&path);
        Ok(path)
    }

    fn delete_file(&mut self) -> Result<()> {
        if self.selected_path.is_file() {
            std::fs::remove_file(&self.selected_path)?
        } else {
            std::fs::remove_dir_all(&self.selected_path)?
        };
        self.select_up();
        self.force_sync();
        Ok(())
    }

    pub fn rename_file(&mut self, name: String) -> Result<()> {
        if let Some(selected) = self.get_selected() {
            let mut new_path = selected.path().clone();
            new_path.pop();
            new_path.push(&name);
            std::fs::rename(selected.path(), &new_path)?;
            selected.update_path(new_path.clone());
            self.selected_path = new_path;
            self.force_sync();
        }
        Ok(())
    }

    pub fn search_paths(&self, pattern: String) -> Vec<PathBuf> {
        let tree = self.tree.clone();
        let mut buffer = Vec::new();
        tree.search_in_paths(pattern.as_str(), &mut buffer);
        buffer
    }

    pub async fn search_files(&self, pattern: String) -> Vec<(PathBuf, String, usize)> {
        let tree = self.tree.clone();
        let mut buffer = JoinSet::new();
        tree.search_in_files(pattern.into(), &mut buffer);
        let mut results = Vec::new();
        while let Some(result) = buffer.join_next().await {
            if let Ok(result) = result {
                results.extend(result)
            }
        }
        results
    }

    pub fn select_by_path(&mut self, path: &PathBuf) {
        self.state.select(None);
        if self.tree.expand_contained(path) {
            self.state.select(Some(0));
            self.selected_path = path.clone();
            self.force_sync();
        }
    }

    pub fn get_first_selected_folder(&self) -> String {
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

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    fn get_list_items(&self) -> Vec<ListItem<'_>> {
        let mut buffer = Vec::new();
        for tree_ptr in self.tree_ptrs.iter() {
            if let Some(tree_path) = unsafe { tree_ptr.as_ref() } {
                buffer.push(ListItem::new(tree_path.display()))
            }
        }
        buffer
    }

    fn get_selected(&self) -> Option<&mut TreePath> {
        unsafe { self.tree_ptrs.get(self.state.selected()?)?.as_mut() }
    }

    fn sync(&mut self) {
        if self.clock.elapsed() >= TICK {
            self.force_sync();
        }
    }

    fn force_sync(&mut self) {
        self.tree.sync();
        self.tree.sync_flat_ptrs(&mut self.tree_ptrs, &self.ignore_base_paths);
        self.fix_select_by_path();
        self.clock = Instant::now();
    }

    fn fix_select_by_path(&mut self) {
        if let Some(selected) = self.get_selected() {
            if &self.selected_path != selected.path() {
                self.state.select(None);
                for (idx, tree_path) in self.tree_ptrs.iter_mut().flat_map(|ptr| unsafe { ptr.as_mut() }).enumerate() {
                    if tree_path.path() == &self.selected_path {
                        self.state.select(Some(idx));
                        break;
                    }
                }
                if self.state.selected().is_none() {
                    self.selected_path = PathBuf::from("./");
                }
            }
        }
    }

    fn unsafe_select(&mut self, idx: usize) {
        self.state.select(Some(idx));
        if let Some(selected) = self.get_selected() {
            self.selected_path = selected.path().clone();
        }
    }
}
