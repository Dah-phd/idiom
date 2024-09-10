use crate::{
    configs::{KeyMap, TreeAction, TreeKeyMap},
    error::{IdiomError, IdiomResult},
    render::{
        backend::{Backend, BackendProtocol},
        layout::Rect,
        state::State,
    },
    tree::TreePath,
};
use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyEvent};
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
}

impl Args {
    pub fn get_path(self) -> IdiomResult<Option<PathBuf>> {
        match self.path {
            Some(rel_path) => {
                let path = rel_path.canonicalize()?;

                match path.is_dir() {
                    true => {
                        std::env::set_current_dir(path)?;
                        Ok(None)
                    }
                    false => {
                        if let Some(path) = path.parent() {
                            std::env::set_current_dir(path)?;
                        }
                        Ok(Some(path))
                    }
                }
            }
            None => Ok(None),
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
    pub fn select(backend: &mut Backend) -> IdiomResult<Option<PathBuf>> {
        let home = dirs::home_dir().ok_or(IdiomError::io_err("Filed to find home dir!"))?.canonicalize()?;
        std::env::set_current_dir(&home)?;
        let config = KeyMap::new().unwrap_or_default();
        backend.hide_cursor();
        let rect = Backend::screen()?;
        let path_str = home.display().to_string();
        let display_offset = path_str.split(std::path::MAIN_SEPARATOR).count() * 2;
        let tree = TreePath::from_path(home.clone());
        let mut tree = Self {
            state: State::new(),
            key_map: config.tree_key_map(),
            display_offset,
            selected_path: home,
            tree,
            rebuild: true,
        };
        tree.render_stateless(rect, backend);
        loop {
            if crossterm::event::poll(MIN_FRAMERATE)? {
                match crossterm::event::read()? {
                    Event::Key(KeyEvent { code: KeyCode::Char('q') | KeyCode::Esc, .. }) => {
                        return Err(IdiomError::any("Exit during tree select!"));
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char(' '), .. }) => {
                        if tree.selected_path.is_dir() {
                            std::env::set_current_dir(&tree.selected_path)?;
                            return Ok(None);
                        }
                        if let Some(parent) = tree.selected_path.parent() {
                            std::env::set_current_dir(parent)?;
                        }
                        return Ok(Some(tree.selected_path));
                    }
                    Event::Key(key) => {
                        tree.map_stateless(&key);
                    }
                    _ => {}
                }
            }
            tree.fast_render_stateless(rect, backend);
        }
    }

    pub fn expand_dir_or_get_path(&mut self) -> Option<PathBuf> {
        let tree_path = self.tree.get_mut_from_inner(self.state.selected)?;
        if tree_path.path().is_dir() {
            tree_path.expand();
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

    pub fn render_stateless(&mut self, rect: Rect, backend: &mut Backend) {
        let mut iter = self.tree.iter();
        iter.next();
        let mut lines = rect.into_iter();
        for (idx, tree_path) in iter.enumerate().skip(self.state.at_line) {
            let line = match lines.next() {
                Some(line) => line,
                None => return,
            };
            if idx == self.state.selected {
                tree_path.render_styled(self.display_offset, line, self.state.highlight, backend);
            } else {
                tree_path.render(self.display_offset, line, backend);
            }
        }
        for line in lines {
            line.render_empty(backend);
        }
    }

    pub fn fast_render_stateless(&mut self, rect: Rect, backend: &mut Backend) {
        if self.rebuild {
            self.rebuild = false;
            self.render_stateless(rect, backend);
        };
    }

    fn select_up(&mut self) {
        let tree_len = self.tree.len() - 1;
        if tree_len == 0 {
            return;
        }
        self.state.prev(tree_len);
        self.unsafe_set_path();
    }

    fn select_down(&mut self) {
        let tree_len = self.tree.len() - 1;
        if tree_len == 0 {
            return;
        }
        self.state.next(tree_len);
        self.unsafe_set_path();
    }

    fn unsafe_set_path(&mut self) {
        self.rebuild = true;
        if let Some(selected) = self.tree.get_mut_from_inner(self.state.selected) {
            self.selected_path = selected.path().clone();
        }
    }

    fn map_stateless(&mut self, key: &KeyEvent) -> bool {
        if let Some(action) = self.key_map.map(key) {
            match action {
                TreeAction::Up => self.select_up(),
                TreeAction::Down => self.select_down(),
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
