mod app;
mod cli;
mod configs;
mod embeded_term;
mod embeded_tui;
mod error;
mod global_state;
mod lsp;
mod popups;
mod syntax;
mod tree;
mod utils;
mod workspace;

use app::app;
use clap::Parser;
use cli::Args;
use error::IdiomResult;
use global_state::CrossTerm;
use idiom_ui::backend::Backend;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> IdiomResult<()> {
    let args = Args::parse();
    let mut backend = CrossTerm::init();
    let open_file = args.collect(&mut backend)?;
    app(open_file, backend).await
}
