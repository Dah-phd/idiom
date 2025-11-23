mod actions;
mod app;
mod cli;
mod configs;
mod cursor;
mod editor;
mod editor_line;
mod embeded_term;
mod embeded_tui;
mod error;
mod ext_tui;
mod global_state;
mod lsp;
mod popups;
mod session;
mod syntax;
mod tree;
mod utils;
mod workspace;

use app::app;
use clap::Parser;
use cli::Args;
use error::IdiomResult;
use ext_tui::CrossTerm;
use idiom_tui::Backend;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> IdiomResult<()> {
    let args = Args::parse();
    let mut backend = CrossTerm::init();
    let open_file = args.collect(&mut backend)?;
    app(open_file, backend).await
}
