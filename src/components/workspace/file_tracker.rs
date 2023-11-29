use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::{
    collections::HashMap,
    convert::Infallible,
    path::{Path, PathBuf},
};
use tokio::time::sleep;
use tokio::{sync::Mutex, task::JoinHandle};

pub struct FileTracker {
    registered: Arc<Mutex<Vec<FileMetadata>>>,
    inner: JoinHandle<Infallible>,
}

impl Default for FileTracker {
    fn default() -> Self {
        let registered = Arc::default();
        let tracked_files = Arc::clone(&registered);
        Self {
            registered,
            inner: tokio::spawn(async move {
                loop {
                    let mut files = tracked_files.lock().await;
                    for metadata in files.iter_mut() {
                        metadata.update();
                    }
                    drop(files);
                    sleep(Duration::from_secs(1)).await;
                }
            }),
        }
    }
}

impl FileTracker {
    pub async fn register(&mut self, path: &PathBuf) {
        let mut registered = self.registered.lock().await;
        if registered.iter().find(|meta| &meta.path == path).is_some() {
            return;
        }
        registered.push(path.into());
    }

    pub async fn unregister(&mut self, path: &PathBuf) {
        self.registered.lock().await.retain(|metadata| &metadata.path != path);
    }

    pub fn is_updated(&mut self, path: &PathBuf) -> bool {
        if let Ok(mut guard) = self.registered.try_lock() {
            if let Some(metadata) = guard.iter_mut().find(|meta| &meta.path == path) {
                return std::mem::take(&mut metadata.updated);
            }
        }
        false
    }
}

pub struct FileMetadata {
    pub path: PathBuf,
    pub updated: bool,
    pub timestamp: Option<SystemTime>,
}

impl From<&PathBuf> for FileMetadata {
    fn from(path: &PathBuf) -> Self {
        Self { path: path.to_owned(), updated: false, timestamp: derive_timestamp(path).ok() }
    }
}

impl FileMetadata {
    fn update(&mut self) {
        if self.updated {
            self.timestamp = derive_timestamp(&self.path).ok();
            return;
        }
        if let Ok(time) = derive_timestamp(&self.path) {
            if let Some(old_time) = self.timestamp {
                self.updated = time == old_time;
            } else {
                self.updated = true;
            }
            self.timestamp.replace(time);
        } else if self.timestamp.take().is_some() {
            self.updated = true;
        }
    }
}

fn derive_timestamp(path: &PathBuf) -> std::io::Result<SystemTime> {
    std::fs::metadata(path)?.modified()
}
