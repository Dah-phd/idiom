use dirs::executable_dir;
use std::path::PathBuf;
use std::path::MAIN_SEPARATOR;

pub fn try_autocomplete(cmd: &str) -> Option<String> {
    path_finder(cmd)
}

fn path_finder(path_fragment: &str) -> Option<String> {
    let path_fragment = if !path_fragment.starts_with(MAIN_SEPARATOR) && !path_fragment.starts_with("./") {
        format!("./{path_fragment}")
    } else {
        path_fragment.to_owned()
    };
    let path_fragment_buf = PathBuf::from(&path_fragment);
    if let Some(parent) = path_fragment_buf.parent() {
        for path in std::fs::read_dir(parent).ok()?.flatten() {
            let mut derived_path_string = path.path().display().to_string();
            if path.path().is_dir() {
                derived_path_string.push(MAIN_SEPARATOR);
            };
            if derived_path_string.starts_with(&path_fragment) {
                return Some(derived_path_string);
            };
        }
    }
    None
}

/// in progress
#[allow(dead_code)]
fn derive_executables(cmd: &str) -> Option<String> {
    if cmd.split(' ').count() != 1 {
        return None;
    }
    for bin in std::fs::read_dir(executable_dir()?).ok()?.flatten() {
        let bin = bin.path().display().to_string();
        if bin.starts_with(cmd) {
            return Some(bin);
        };
    }
    None
}
