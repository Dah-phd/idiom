mod app;
mod cli;
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
use cli::cli;
use error::IdiomResult;
use render::backend::{Backend, BackendProtocol};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> IdiomResult<()> {
    let mut backend = Backend::init();
    let cli_result = cli(&mut backend);
    app(cli_result, backend).await?;
    Backend::exit()?;
    Ok(())
}
