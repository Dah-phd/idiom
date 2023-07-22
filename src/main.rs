mod app;
mod components;
mod lsp;
mod messages;
mod syntax;
mod utils;

use std::path::PathBuf;

use app::app;

use tui::backend::CrosstermBackend;
use tui::Terminal;

fn prep(out: &mut impl std::io::Write) -> std::io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        out,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::style::ResetColor,
    )?;
    Ok(())
}

fn graceful_exit(out: &mut impl std::io::Write) -> std::io::Result<()> {
    crossterm::execute!(
        out,
        crossterm::style::ResetColor,
        crossterm::terminal::LeaveAlternateScreen,
    )?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

fn cli() -> Option<PathBuf> {
    let argv: Vec<String> = std::env::args().collect();
    let path = PathBuf::from(argv.get(1)?).canonicalize().ok()?;
    if path.is_file() {
        std::env::set_current_dir(path.parent()?).ok()?;
        return Some(path);
    } else {
        std::env::set_current_dir(path).ok()?;
    }
    None
}

async fn debug() {}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    debug().await;
    let out = std::io::stdout();
    let mut terminal = Terminal::new(CrosstermBackend::new(&out)).expect("should not fail!");
    prep(&mut terminal.backend_mut())?;
    app(&mut terminal, cli()).await?;
    graceful_exit(&mut terminal.backend_mut())
}
