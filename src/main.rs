mod state;
// mod events;
mod messages;
mod screen;
use screen::app;
use state::State;

use std::io::{stdout, Write};
use std::sync::{Arc, RwLock};

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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let out = stdout();
    let state = Arc::new(RwLock::new(State::new()));
    let mut terminal = Terminal::new(CrosstermBackend::new(&out)).expect("should not fail!");
    prep(&mut terminal.backend_mut())?;
    app(&mut terminal, state.clone())?;
    graceful_exit(&mut terminal.backend_mut())
}
