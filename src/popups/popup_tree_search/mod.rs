mod search_files;
mod search_paths;
pub use search_files::ActiveFileSearch;
pub use search_paths::ActivePathSearch;

use super::{Components, Popup, Status};
use std::time::Duration;

const WAIT_ON_UPDATE: Duration = Duration::from_millis(100);
