use crate::{
    configs::{KeyMap, TreeAction, TreeKeyMap},
    error::{IdiomError, IdiomResult},
    ext_tui::{CrossTerm, State},
    session::restore_last_sesson,
    tree::TreePath,
};
use clap::Parser;
use crossterm::{
    event::{Event, KeyCode, KeyEvent},
    style::ContentStyle,
};
use idiom_tui::{layout::Rect, Backend};
use std::{path::PathBuf, time::Duration};

const MIN_FRAMERATE: Duration = Duration::from_millis(8);

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Optinal path to open on start
    path: Option<PathBuf>,
    /// Run in select mode opening basic file tree from HOME dir (ignores provided PATH args)
    #[arg(short, long)]
    pub select: bool,
    /// Attempts to restore last saved session
    #[arg(short, long)]
    pub restore: bool,
}

impl Args {
    pub fn collect(self, backend: &mut CrossTerm) -> IdiomResult<Option<PathBuf>> {
        if self.restore {
            let path = restore_last_sesson()?;
            std::env::set_current_dir(path)?;
            return Ok(None);
        }
        match self.path {
            Some(rel_path) => {
                let path = rel_path.canonicalize()?;
                match self.select {
                    true => match path.is_dir() {
                        true => TreeSeletor::select(backend, path),
                        false => {
                            let parent_path = match path.parent() {
                                None => {
                                    return Err(IdiomError::io_not_found(format!(
                                        "Unable to derive parent directory of {path:?}!",
                                    )))
                                }
                                Some(path) => path.to_owned(),
                            };
                            TreeSeletor::select(backend, parent_path)
                        }
                    },
                    false => match path.is_dir() {
                        true => {
                            std::env::set_current_dir(&path)?;
                            Ok(None)
                        }
                        false => {
                            if let Some(path) = path.parent() {
                                std::env::set_current_dir(path)?;
                            }
                            Ok(Some(path))
                        }
                    },
                }
            }
            None => match self.select {
                true => TreeSeletor::select_home(backend),
                false => Ok(None),
            },
        }
    }
}

pub struct TreeSeletor {
    pub key_map: TreeKeyMap,
    state: State,
    selected_path: PathBuf,
    tree: TreePath,
    display_offset: usize,
    rebuild: bool,
}

/// Stateless calls
impl TreeSeletor {
    pub fn new(selected_path: PathBuf) -> IdiomResult<Self> {
        std::env::set_current_dir(&selected_path)?;
        let key_map = KeyMap::new().unwrap_or_default().tree_key_map();
        let path_str = selected_path.display().to_string();
        let display_offset = path_str.split(std::path::MAIN_SEPARATOR).count() * 2;
        let tree = TreePath::from_path(selected_path.clone()).unwrap();
        Ok(Self { state: State::new(), key_map, display_offset, selected_path, tree, rebuild: true })
    }

    pub fn select(backend: &mut CrossTerm, path: PathBuf) -> IdiomResult<Option<PathBuf>> {
        let tree_selector = Self::new(path)?;
        tree_selector.run(backend)
    }

    pub fn select_home(backend: &mut CrossTerm) -> IdiomResult<Option<PathBuf>> {
        let home_path = dirs::home_dir().ok_or(IdiomError::io_not_found("Filed to find home dir!"))?.canonicalize()?;
        Self::select(backend, home_path)
    }

    fn run(mut self, backend: &mut CrossTerm) -> IdiomResult<Option<PathBuf>> {
        let rect = CrossTerm::screen()?;
        self.render_stateless(rect, backend);
        let limit = rect.height as usize;
        loop {
            if crossterm::event::poll(MIN_FRAMERATE)? {
                match crossterm::event::read()? {
                    Event::Key(KeyEvent { code: KeyCode::Char('q') | KeyCode::Esc, .. }) => {
                        return Err(IdiomError::any("Exit during tree select!"));
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char(' '), .. }) => {
                        if self.selected_path.is_dir() {
                            std::env::set_current_dir(&self.selected_path)?;
                            return Ok(None);
                        }
                        if let Some(parent) = self.selected_path.parent() {
                            std::env::set_current_dir(parent)?;
                        }
                        return Ok(Some(self.selected_path));
                    }
                    Event::Key(key) => {
                        self.map_stateless(&key, limit);
                    }
                    _ => {}
                }
            }
            self.fast_render_stateless(rect, backend);
        }
    }

    pub fn expand_dir_or_get_path(&mut self) -> Option<PathBuf> {
        let tree_path = self.tree.get_mut_from_inner(self.state.selected)?;
        if tree_path.path().is_dir() {
            let _ = tree_path.expand();
            self.rebuild = true;
            None
        } else {
            Some(tree_path.path().clone())
        }
    }

    fn shrink(&mut self) {
        if let Some(tree_path) = self.tree.get_mut_from_inner(self.state.selected) {
            tree_path.take_tree();
            self.rebuild = true;
        }
    }

    pub fn render_stateless(&mut self, rect: Rect, backend: &mut CrossTerm) {
        let mut iter = self.tree.iter();
        iter.next();
        let mut lines = rect.into_iter();
        for (idx, tree_path) in iter.enumerate().skip(self.state.at_line) {
            let line = match lines.next() {
                Some(line) => line,
                None => return,
            };
            if idx == self.state.selected {
                tree_path.render(self.display_offset, line, self.state.highlight, backend);
            } else {
                tree_path.render(self.display_offset, line, ContentStyle::default(), backend);
            }
        }
        for line in lines {
            line.render_empty(backend);
        }
    }

    pub fn fast_render_stateless(&mut self, rect: Rect, backend: &mut CrossTerm) {
        if self.rebuild {
            self.rebuild = false;
            self.render_stateless(rect, backend);
        };
    }

    fn select_up(&mut self, limit: usize) {
        let tree_len = self.tree.len() - 1;
        if tree_len == 0 {
            return;
        }
        self.state.prev(tree_len);
        self.state.update_at_line(limit);
        self.unsafe_set_path();
    }

    fn select_down(&mut self, limit: usize) {
        let tree_len = self.tree.len() - 1;
        if tree_len == 0 {
            return;
        }
        self.state.next(tree_len);
        self.state.update_at_line(limit);
        self.unsafe_set_path();
    }

    fn unsafe_set_path(&mut self) {
        self.rebuild = true;
        if let Some(selected) = self.tree.get_mut_from_inner(self.state.selected) {
            self.selected_path = selected.path().clone();
        }
    }

    fn map_stateless(&mut self, key: &KeyEvent, limit: usize) -> bool {
        if let Some(action) = self.key_map.map(key) {
            match action {
                TreeAction::Up => self.select_up(limit),
                TreeAction::Down => self.select_down(limit),
                TreeAction::Shrink => self.shrink(),
                TreeAction::Expand => {
                    let _ = self.expand_dir_or_get_path();
                }
                _ => {}
            }
            return true;
        }
        false
    }
}
