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
use clap::Parser;
use cli::{Args, TreeSeletor};
use error::IdiomResult;
use render::backend::{Backend, BackendProtocol};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> IdiomResult<()> {
    let args = Args::parse();
    let mut backend = Backend::init();
    let open_file = match args.select {
        false => args.get_path()?,
        true => TreeSeletor::select(&mut backend)?,
    };
    app(open_file, backend).await
}
