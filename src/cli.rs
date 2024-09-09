use crate::{
    configs::KeyMap,
    error::{IdiomError, IdiomResult},
    render::backend::{Backend, BackendProtocol},
    tree::Tree,
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use std::{
    path::{PathBuf, MAIN_SEPARATOR},
    time::Duration,
};

const MIN_FRAMERATE: Duration = Duration::from_millis(8);
const SELECT_PAT: [&str; 2] = ["-S", "--select"];

pub fn cli(backend: &mut Backend) -> Option<PathBuf> {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    let mut selected_path = None;

    for arg in argv.iter() {
        if SELECT_PAT.contains(&arg.as_str()) {
            let path = tree_selector(backend).unwrap();
            selected_path.replace(path);
            break;
        }
    }

    let path = match selected_path {
        Some(path) => path,
        None => PathBuf::from(argv.first()?).canonicalize().expect("Unable to locate selected dir!"),
    };

    if path.is_file() {
        std::env::set_current_dir(path.parent()?).expect("Failed to move to selected dir!");
        if let Some(Some(path_ptr)) = argv.first().map(|s| s.split(MAIN_SEPARATOR).last()) {
            return Some(PathBuf::from(format!("./{}", path_ptr)));
        }
        return Some(path);
    } else {
        std::env::set_current_dir(path).expect("Failed to move to selected dir!");
    }
    None
}

fn tree_selector(backend: &mut Backend) -> IdiomResult<PathBuf> {
    let home = dirs::home_dir().ok_or(IdiomError::io_err("Filed to find home dir!"))?;
    std::env::set_current_dir(home)?;
    let config = KeyMap::new().unwrap_or_default();
    backend.hide_cursor();
    let rect = Backend::screen()?;
    let mut tree = Tree::new(config.tree_key_map());
    tree.render_stateless(rect, backend);
    loop {
        if crossterm::event::poll(MIN_FRAMERATE)? {
            match crossterm::event::read()? {
                Event::Key(KeyEvent { code: KeyCode::Char('q') | KeyCode::Esc, .. }) => {
                    return Err(IdiomError::any("Exit during tree select!"));
                }
                Event::Key(KeyEvent { code: KeyCode::Char(' '), .. }) => return Ok(tree.unwrap_selected()),
                Event::Key(key) => {
                    tree.map_stateless(&key);
                }
                _ => {}
            }
        }
        tree.fast_render_stateless(rect, backend);
    }
}
