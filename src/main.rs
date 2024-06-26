mod app;
mod configs;
mod error;
mod global_state;
mod lsp;
mod popups;
mod render;
mod runner;
mod syntax;
mod tree;
mod utils;
mod workspace;

use app::app;
use error::IdiomResult;
use render::backend::{Backend, BackendProtocol};
use std::path::{PathBuf, MAIN_SEPARATOR};

fn cli() -> Option<PathBuf> {
    let argv: Vec<String> = std::env::args().collect();
    let path = PathBuf::from(argv.get(1)?).canonicalize().ok()?;
    if path.is_file() {
        std::env::set_current_dir(path.parent()?).ok()?;
        if let Some(Some(path_ptr)) = argv.get(1).map(|s| s.split(MAIN_SEPARATOR).last()) {
            return Some(PathBuf::from(format!("./{}", path_ptr)));
        }
        return Some(path);
    } else {
        std::env::set_current_dir(path).ok()?;
    }
    None
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> IdiomResult<()> {
    app(cli(), Backend::init()).await?;
    Backend::exit()?;
    Ok(())
}
