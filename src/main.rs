mod app;
mod configs;
mod footer;
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

use anyhow::Result;
use std::path::{PathBuf, MAIN_SEPARATOR};

fn init_terminal() -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::style::ResetColor,
        crossterm::event::EnableMouseCapture,
    )?;

    // loading panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        graceful_exit().unwrap();
        original_hook(panic);
    }));
    Ok(())
}

fn graceful_exit() -> Result<()> {
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::style::ResetColor,
        crossterm::event::DisableMouseCapture,
        crossterm::cursor::Show,
    )?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

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
async fn main() -> Result<()> {
    init_terminal()?;
    app(cli()).await?;
    graceful_exit()
}
