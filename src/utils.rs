use std::{
    ops::{Add, Sub},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    error::{IdiomError, IdiomResult},
    workspace::line::EditorLine,
};

pub fn trim_start_inplace(line: &mut EditorLine) -> usize {
    if let Some(idx) = line.to_string().find(|c: char| !c.is_whitespace() && c != '\t') {
        line.replace_till(idx, "");
        return idx;
    };
    0
}

pub fn split_arc<T: Default>() -> (Arc<T>, Arc<T>) {
    let arc = Arc::default();
    let clone = Arc::clone(&arc);
    (arc, clone)
}

pub fn get_nested_paths(path: &PathBuf) -> IdiomResult<impl Iterator<Item = PathBuf>> {
    Ok(std::fs::read_dir(path)?.flatten().map(|p| p.path()))
}

pub fn build_file_or_folder(base_path: PathBuf, add: &str) -> IdiomResult<PathBuf> {
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
                return Err(IdiomError::io_exists("File already exists!"));
            }
        }
        std::fs::write(&path, "")?;
    }

    Ok(path)
}

pub fn to_relative_path(target_dir: &Path) -> IdiomResult<PathBuf> {
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
        Err(IdiomError::io_other("Empty buffer!"))
    } else {
        Ok(result)
    }
}

pub fn to_canon_path(target_dir: &Path) -> IdiomResult<PathBuf> {
    Ok(target_dir.canonicalize()?)
}

pub struct TrackedList<T> {
    inner: Vec<T>,
    updated: bool,
}

impl<T> TrackedList<T> {
    #[inline(always)]
    pub fn from(inner: Vec<T>) -> Self {
        Self { inner, updated: true }
    }

    #[inline(always)]
    pub fn new() -> Self {
        Self { inner: Vec::new(), updated: true }
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn first(&self) -> Option<&T> {
        self.inner.first()
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.updated = true;
        self.inner.first_mut()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline(always)]
    pub fn get_mut_no_update(&mut self, index: usize) -> Option<&mut T> {
        self.inner.get_mut(index)
    }

    #[inline(always)]
    pub fn collect_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn check_status(&self) -> bool {
        self.updated
    }

    #[inline(always)]
    pub fn mark_updated(&mut self) {
        self.updated = true;
    }

    #[inline(always)]
    pub fn insert(&mut self, index: usize, element: T) {
        self.updated = true;
        self.inner.insert(index, element)
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }

    #[inline(always)]
    pub fn inner(&self) -> &Vec<T> {
        &self.inner
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut Vec<T> {
        self.updated = true;
        &mut self.inner
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn inner_mut_no_update(&mut self) -> &mut Vec<T> {
        &mut self.inner
    }

    #[inline(always)]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn iter_if_updated(&mut self) -> Option<std::slice::Iter<'_, T>> {
        if !self.collect_status() {
            return None;
        }
        Some(self.iter())
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.updated = true;
        self.inner.iter_mut()
    }

    #[inline(always)]
    pub fn find<P>(&mut self, mut predicate: P) -> Option<&mut T>
    where
        P: FnMut(&T) -> bool,
    {
        for element in self.inner.iter_mut() {
            if (predicate)(element) {
                self.updated = true;
                return Some(element);
            }
        }
        None
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.updated = true;
        self.inner.get_mut(index)
    }

    #[inline(always)]
    pub fn push(&mut self, element: T) {
        self.updated = true;
        self.inner.push(element);
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        let result = self.inner.pop();
        if result.is_some() {
            self.updated = true;
        }
        result
    }

    #[inline(always)]
    pub fn remove(&mut self, index: usize) -> T {
        self.updated = true;
        self.inner.remove(index)
    }
}

impl<T> From<Vec<T>> for TrackedList<T> {
    fn from(value: Vec<T>) -> Self {
        Self::from(value)
    }
}

#[derive(Clone, Copy)]
pub enum Offset {
    Pos(usize),
    Neg(usize),
}

impl Offset {
    pub fn offset(self, val: usize) -> usize {
        match self {
            Self::Pos(numba) => val + numba,
            Self::Neg(numba) => val.saturating_sub(numba),
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
