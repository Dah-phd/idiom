use crate::{
    configs::CONFIG_FOLDER,
    global_state::{GlobalState, WorkspaceEvent},
};
use dirs::config_dir;

pub fn load_cfg(f: &str, gs: &mut GlobalState) -> Option<String> {
    let mut path = match config_dir() {
        Some(path) => path,
        None => {
            return Some("Unable to resolve config dir".to_owned());
        }
    };
    path.push(CONFIG_FOLDER);
    path.push(f);
    gs.workspace.push_back(WorkspaceEvent::Open(path, 0));
    None
}
