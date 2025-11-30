use crate::{
    editor_line::EditorLine,
    error::{IdiomError, IdiomResult},
};
use std::{
    ops::{Add, Sub},
    path::{Path, PathBuf},
    sync::Arc,
};

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

#[derive(Debug, PartialEq, Clone, Copy)]
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

/// provides information about direction of sequence, against value ordering
/// espectially useful for 2 values
///
/// Example with selecting of value, in order to be able to parse text effectively
/// it makes sense to start from smaller value, in certain cases also the information
/// regarding the original state is need - that is the used case for the Direction.
#[derive(Debug, PartialEq)]
pub enum Direction {
    Normal,
    Reversed,
}

impl Direction {
    pub fn is_reversed(&self) -> bool {
        matches!(self, Self::Reversed)
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Reversed)
    }

    /// will apply the callback based on the order struct
    /// if normal callback(x, y)
    /// if reversed callback(y, x)
    pub fn apply_ordered<T, R, F>(&self, x: T, y: T, callback: F) -> R
    where
        F: FnOnce(T, T) -> R,
    {
        match self {
            Self::Normal => (callback)(x, y),
            Self::Reversed => (callback)(y, x),
        }
    }
}

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

#[cfg(test)]
pub mod test {
    use super::Direction;
    use std::path::{Path, PathBuf};

    pub struct TempDir {
        inner: PathBuf,
    }

    impl TempDir {
        pub fn new(title: &str) -> std::io::Result<Self> {
            let mut inner = PathBuf::from(".").canonicalize()?;
            inner.push(format!("tmp-{title}"));
            std::fs::create_dir(&inner).map(|_| Self { inner })
        }

        pub fn path(&self) -> &Path {
            &self.inner
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            _ = std::fs::remove_dir_all(self.path());
        }
    }

    #[test]
    fn temp_dir() {
        let temp_dir = TempDir::new("shoulwork").unwrap();
        let tp = temp_dir.path().to_owned();
        assert!(tp.exists());
        drop(temp_dir);
        assert!(!tp.exists());
    }

    #[test]
    fn diction_apply() {
        let norm = Direction::Normal;
        let revr = Direction::Reversed;
        assert!(norm.apply_ordered(1, 2, |x, y| { x < y }));
        assert!(revr.apply_ordered(1, 2, |x, y| { x > y }));

        let outer_val = 30;
        let expected = 23;

        assert_eq!(norm.apply_ordered(3, 10, |x, y| (outer_val + x) - y), expected);
        assert_eq!(revr.apply_ordered(10, 3, |x, y| (outer_val + x) - y), expected);
    }
}
