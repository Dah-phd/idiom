mod tree_paths;
use crate::{
    global_state::{GlobalState, Mode},
    utils::{build_file_or_folder, to_relative_path},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tree_paths::TreePath;

const TICK: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct Tree {
    pub on_open_tabs: bool,
    active: bool,
    state: ListState,
    selected_path: PathBuf,
    tree: TreePath,
    tree_ptrs: Vec<*mut TreePath>,
    clock: Instant,
}

impl Tree {
    pub fn new(active: bool) -> Self {
        let mut tree = TreePath::default();
        let mut tree_ptrs = Vec::new();
        tree.sync_flat_ptrs(&mut tree_ptrs);
        Self {
            active,
            state: ListState::default(),
            selected_path: PathBuf::from("./"),
            tree,
            tree_ptrs,
            clock: Instant::now(),
            on_open_tabs: false,
        }
    }

    pub fn render_with_remainder(&mut self, frame: &mut Frame, screen: Rect, gs: &mut GlobalState) -> Rect {
        if matches!(gs.mode, Mode::Insert) && !self.active {
            return screen;
        }
        let areas = Layout::new(Direction::Horizontal, [Constraint::Percentage(15), Constraint::Min(2)]).split(screen);

        self.sync();

        let list_items = self
            .tree_ptrs
            .iter()
            .flat_map(|ptr| unsafe { ptr.as_ref() }.map(|tree_path| ListItem::new(tree_path.display())))
            .collect::<Vec<ListItem<'_>>>();

        let tree = List::new(list_items)
            .block(Block::default().borders(Borders::ALL).title("Explorer"))
            .highlight_style(Style { add_modifier: Modifier::REVERSED, ..Default::default() });

        frame.render_stateful_widget(tree, areas[0], &mut self.state);
        areas[1]
    }

    pub fn map(&mut self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('w' | 'W') => self.select_up(),
            KeyCode::Down | KeyCode::Char('s' | 'S') => self.select_down(),
            KeyCode::Left => self.shrink(),
            KeyCode::Char('d' | 'D') if !key.modifiers.contains(KeyModifiers::CONTROL) => self.shrink(),
            KeyCode::Delete if key.modifiers == KeyModifiers::SHIFT => {
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
        self.state.select(None);
        if self.tree.expand_contained(path) {
            self.state.select(Some(0));
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

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    pub fn get_selected(&self) -> Option<&mut TreePath> {
        unsafe { self.tree_ptrs.get(self.state.selected()?)?.as_mut() }
    }

    fn sync(&mut self) {
        if self.clock.elapsed() >= TICK {
            self.force_sync();
        }
    }

    fn force_sync(&mut self) {
        self.tree.sync_base();
        self.tree.sync_flat_ptrs(&mut self.tree_ptrs);
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
