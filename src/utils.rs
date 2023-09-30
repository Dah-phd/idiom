use crate::components::editor::Offset;
use anyhow::{anyhow, Result};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

pub fn trim_start_inplace(line: &mut String) -> Offset {
    if let Some(idx) = line.find(|c: char| !c.is_whitespace()) {
        line.replace_range(..idx, "");
        return Offset::Neg(idx);
    };
    Offset::Pos(0)
}

pub fn trim_start(mut line: String) -> String {
    trim_start_inplace(&mut line);
    line
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

pub fn centered_rect_static(h: u16, v: u16, rect: Rect) -> Rect {
    let h_diff = rect.width.checked_sub(h).unwrap_or_default() / 2;
    let v_diff = rect.height.checked_sub(v).unwrap_or_default() / 2;
    let first_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(v_diff),
            Constraint::Min(v),
            Constraint::Length(v_diff),
        ])
        .split(rect);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(h_diff),
            Constraint::Min(h),
            Constraint::Length(h_diff),
        ])
        .split(first_split[1])[1]
}

pub fn find_code_blocks(buffer: &mut Vec<(usize, String)>, content: &[String], pattern: &str) {
    let mut content_iter = content.iter().enumerate().peekable();
    while let Some((idx, line)) = content_iter.next() {
        if !line.contains(pattern) {
            continue;
        }
        let mut line = line.to_owned();
        let white_chars_len = trim_start_inplace(&mut line).unwrap();
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
