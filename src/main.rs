mod app;
mod configs;
mod footer;
mod global_state;
mod lsp;
mod popups;
mod syntax;
mod terminal;
mod tree;
mod utils;
mod widgests;
mod workspace;

use app::app;

use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::Stdout,
    path::{PathBuf, MAIN_SEPARATOR},
};

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::style::ResetColor,
        crossterm::cursor::SetCursorStyle::BlinkingBlock,
    )?;

    // loading panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        graceful_exit().unwrap();
        original_hook(panic);
    }));

    Ok(terminal)
}

fn graceful_exit() -> Result<()> {
    crossterm::execute!(std::io::stdout(), crossterm::style::ResetColor, crossterm::terminal::LeaveAlternateScreen,)?;
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
    let terminal = init_terminal()?;
    app(terminal, cli()).await?;
    graceful_exit()
}
