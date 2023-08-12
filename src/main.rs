mod app;
mod components;
mod configs;
mod lsp;
mod syntax;
mod utils;

use std::path::PathBuf;

use app::app;

use lsp::LSP;

use anyhow::Result;
use tui::backend::CrosstermBackend;
use tui::Terminal;

fn prep(out: &mut impl std::io::Write) -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        out,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::style::ResetColor,
        crossterm::cursor::SetCursorStyle::BlinkingBar,
    )?;
    Ok(())
}

fn graceful_exit(out: &mut impl std::io::Write) -> Result<()> {
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

async fn nop() {
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await
}

async fn debug() -> usize {
    let mut lsp = LSP::from(&configs::FileType::Rust).await.unwrap();
    let p: PathBuf = "src/main.rs".into();
    nop().await;
    println!("{:?}", lsp.get(0));
    println!("{:?}", lsp.initialized().await);
    nop().await;
    nop().await;
    println!("open file {:?}", lsp.file_did_open(&p).await);
    nop().await;
    println!("request {:?}", lsp.request_signiture_help(&p, 63, 47).await);
    nop().await;
    nop().await;
    println!("{:?}", lsp.responses.lock());
    println!("{:?}", lsp.notifications.lock());
    println!("{:?}", lsp.requests.lock().await);
    let _ = lsp.graceful_exit().await;
    0
}

#[tokio::main]
async fn main() -> Result<()> {
    // if debug().await == 0 {
    //     return Ok(());
    // };
    let out = std::io::stdout();
    let mut terminal = Terminal::new(CrosstermBackend::new(&out)).expect("should not fail!");
    prep(&mut terminal.backend_mut())?;
    app(&mut terminal, cli()).await?;
    graceful_exit(&mut terminal.backend_mut())
}
