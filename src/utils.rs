use crate::components::editor::Offset;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

pub fn trim_start_inplace(line: &mut String) -> Offset {
    if let Some(idx) = line.find(|c: char| !c.is_whitespace()) {
        line.replace_range(..idx, "");
        return Offset::Neg(idx);
    };
    Offset::Pos(0)
}

pub fn get_closing_char(ch: char) -> Option<char> {
    match ch {
        '{' => Some('}'),
        '(' => Some(')'),
        '[' => Some(']'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

pub fn split_arc_mutex<T>(inner: T) -> (Arc<Mutex<T>>, Arc<Mutex<T>>) {
    let arc = Arc::new(Mutex::new(inner));
    let clone = Arc::clone(&arc);
    (arc, clone)
}

pub fn split_arc_mutex_async<T>(inner: T) -> (Arc<tokio::sync::Mutex<T>>, Arc<tokio::sync::Mutex<T>>) {
    let arc = Arc::new(tokio::sync::Mutex::new(inner));
    let clone = Arc::clone(&arc);
    (arc, clone)
}

pub fn into_guard<T>(mutex: &Mutex<T>) -> MutexGuard<T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn get_nested_paths(path: &PathBuf) -> impl Iterator<Item = PathBuf> {
    match std::fs::read_dir(path) {
        Ok(iter) => iter.flatten().map(|p| p.path()),
        Err(_) => panic!(),
    }
}

pub fn build_file_or_folder(base_path: PathBuf, add: &str) -> Result<PathBuf> {
    let mut path = if base_path.is_dir() {
        base_path
    } else if let Some(parent) = base_path.parent() {
        parent.into()
    } else {
        PathBuf::from("./")
    };

    if add.ends_with('/') || add.ends_with(std::path::MAIN_SEPARATOR) {
        path.push(add);
        std::fs::create_dir_all(&path)?;
    } else {
        if add.contains('/') {
            let mut split: Vec<&str> = add.split('/').collect();
            let file_name = split.pop();
            let stem = split.join("/");
            path.push(stem);
            std::fs::create_dir_all(&path)?;
            if let Some(file_name) = file_name {
                path.push(file_name);
                if path.exists() {
                    return Err(anyhow!("File already exists!"));
                }
                std::fs::write(&path, "")?;
            }
        } else {
            if path.exists() {
                return Err(anyhow!("File already exists!"));
            }
            path.push(add);
        }
        std::fs::write(&path, "")?;
    }

    Ok(path)
}
