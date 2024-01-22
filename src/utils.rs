use anyhow::{anyhow, Result};
use ratatui::{
    style::{Modifier, Style},
    widgets::{Block, Borders},
};
use std::{
    ops::{Add, Sub},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

pub const UNDERLINED: Style = Style::new().add_modifier(Modifier::UNDERLINED);
pub const REVERSED: Style = Style::new().add_modifier(Modifier::REVERSED);
pub const BORDERED_BLOCK: Block = Block::new().borders(Borders::all());

pub fn trim_start_inplace(line: &mut String) -> usize {
    if let Some(idx) = line.find(|c: char| !c.is_whitespace() && c != '\t') {
        line.replace_range(..idx, "");
        return idx;
    };
    0
}

pub fn trim_start(mut line: String) -> String {
    trim_start_inplace(&mut line);
    line
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
        }
        std::fs::write(&path, "")?;
    }

    Ok(path)
}

pub fn to_relative_path(target_dir: &Path) -> Result<PathBuf> {
    let cd = std::env::current_dir()?;
    if target_dir.is_relative() {
        return Ok(target_dir.into());
    }
    let mut result = PathBuf::from("./");
    let mut path_before_current_dir = PathBuf::new();
    let mut after_current_dir = false;
    for component in target_dir.components() {
        if after_current_dir {
            result.push(component.as_os_str());
        } else {
            path_before_current_dir.push(component.as_os_str());
        }
        if path_before_current_dir == cd {
            after_current_dir = true;
        }
    }
    if result.to_string_lossy().is_empty() {
        Err(anyhow!("Empty buffer!"))
    } else {
        Ok(result)
    }
}

#[allow(dead_code)]
pub fn find_code_blocks(buffer: &mut Vec<(usize, String)>, content: &[String], pattern: &str) {
    let mut content_iter = content.iter().enumerate().peekable();
    while let Some((idx, line)) = content_iter.next() {
        if !line.contains(pattern) {
            continue;
        }
        let mut line = line.to_owned();
        let white_chars_len = trim_start_inplace(&mut line);
        if let Some((_, next_line)) = content_iter.peek() {
            if let Some(first_non_white) = next_line.find(|c: char| !c.is_whitespace()) {
                if first_non_white >= white_chars_len {
                    line.push('\n');
                    line.push_str(&next_line[white_chars_len..]);
                }
            }
        }
        buffer.push((idx, line));
    }
}

pub enum Offset {
    Pos(usize),
    Neg(usize),
}

impl Offset {
    pub fn offset(self, val: usize) -> usize {
        match self {
            Self::Pos(numba) => val + numba,
            Self::Neg(numba) => val.checked_sub(numba).unwrap_or_default(),
        }
    }
}

impl Add<usize> for Offset {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        match self {
            Self::Pos(numba) => Self::Pos(numba + rhs),
            Self::Neg(numba) => {
                if numba > rhs {
                    Self::Neg(numba - rhs)
                } else {
                    Self::Pos(rhs - numba)
                }
            }
        }
    }
}

impl Sub<usize> for Offset {
    type Output = Offset;
    fn sub(self, rhs: usize) -> Self::Output {
        match self {
            Self::Neg(numba) => Self::Neg(numba + rhs),
            Self::Pos(numba) => {
                if numba > rhs {
                    Self::Pos(numba - rhs)
                } else {
                    Self::Neg(rhs - numba)
                }
            }
        }
    }
}

impl From<usize> for Offset {
    fn from(value: usize) -> Self {
        Self::Pos(value)
    }
}

#[cfg(build = "debug")]
#[allow(unused_must_use)]
pub fn debug_to_file(path: &str, obj: impl Debug) {
    let mut data = std::fs::read_to_string(path).unwrap_or_default();
    data.push_str(&format!("\n{obj:?}"));
    std::fs::write(path, data);
}
