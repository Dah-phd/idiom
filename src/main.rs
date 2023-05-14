mod messages;
mod lsp;
mod app;
mod components;
use app::app;
use lsp::rust::start_lsp;
use lsp_types::request::Initialize;

use std::io::{stdout, Write};

use crossterm::event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use crossterm::execute;
use crossterm::style::ResetColor;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use tui::backend::CrosstermBackend;
use tui::Terminal;

fn prep(out: &mut impl Write) -> std::io::Result<()> {
    enable_raw_mode()?;
    execute!(
        out,
        EnterAlternateScreen,
        ResetColor,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )?;
    Ok(())
}

fn graceful_exit(out: &mut impl Write) -> std::io::Result<()> {
    execute!(out, ResetColor, LeaveAlternateScreen, PopKeyboardEnhancementFlags)?;
    disable_raw_mode()?;
    Ok(())
}

async fn debug() {
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    debug().await;
    let out = stdout();
    let mut terminal = Terminal::new(CrosstermBackend::new(&out)).expect("should not fail!");
    prep(&mut terminal.backend_mut())?;
    app(&mut terminal)?;
    graceful_exit(&mut terminal.backend_mut())
}
