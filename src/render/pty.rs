use crate::{
    error::{IdiomError, IdiomResult},
    render::{
        backend::{Backend, BackendProtocol},
        layout::Rect,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;
use vt100::{Parser, Screen};

struct TrackedParser {
    inner: Parser,
    updated: bool,
}

impl TrackedParser {
    fn new(rows: u16, cols: u16) -> Self {
        Self { inner: Parser::new(rows, cols, 2000), updated: false }
    }

    fn process(&mut self, bytes: &[u8]) {
        self.updated = true;
        self.inner.process(bytes);
    }

    fn new_screen(&mut self) -> Option<Screen> {
        if !self.updated {
            return None;
        }
        self.updated = false;
        Some(self.inner.screen().clone())
    }

    fn screen(&mut self) -> Screen {
        self.updated = false;
        self.inner.screen().clone()
    }
}

/// Run another tui app within the context of idiom
pub struct PtyShell {
    pair: PtyPair,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output_handler: JoinHandle<std::io::Result<()>>,
    output: Arc<Mutex<TrackedParser>>,
    rect: Rect,
    cursor: CursorState,
}

impl PtyShell {
    pub fn default_cmd(rect: Rect) -> IdiomResult<Self> {
        Self::new(CommandBuilder::new_default_prog(), rect)
    }

    pub fn run(cmd: &str, rect: Rect) -> IdiomResult<Self> {
        Self::new(CommandBuilder::new(cmd), rect)
    }

    pub fn new(mut cmd: CommandBuilder, rect: Rect) -> IdiomResult<Self> {
        let system = native_pty_system();
        let size = PtySize::from(rect);
        let pair = system.openpty(size).map_err(IdiomError::any)?;

        cmd.cwd("./");
        let child = pair.slave.spawn_command(cmd).map_err(IdiomError::any)?;
        let writer = pair.master.take_writer().map_err(IdiomError::any)?;
        let mut reader = pair.master.try_clone_reader().map_err(IdiomError::any)?;
        let output = Arc::new(Mutex::new(TrackedParser::new(size.rows, size.cols)));
        let output_writer = Arc::clone(&output);

        let output_handler = tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            let mut processed_buf = Vec::new();
            loop {
                let size = reader.read(&mut buf)?;
                if size == 0 {
                    return Ok(());
                }
                processed_buf.extend_from_slice(&buf[..size]);
                let mut lock = output_writer.lock().expect("lock on PtyShell read");
                lock.process(&processed_buf);
                processed_buf.clear();
            }
        });

        Ok(Self { rect, pair, child, writer, output, output_handler, cursor: CursorState::from(rect) })
    }

    pub fn key_map(&mut self, key: &KeyEvent) -> std::io::Result<()> {
        if let KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. } = key {
            return self.writer.write_all(&[0x3]);
        };
        match key.code {
            KeyCode::Char(ch) => self.writer.write_all(&[ch as u8]),
            KeyCode::Backspace => self.writer.write_all(&[0x8]),
            KeyCode::Tab => self.writer.write_all(&[0x9]),
            KeyCode::Enter => self.writer.write_all(&[0xD]),
            KeyCode::Delete => self.writer.write_all(&[0x7F]),
            KeyCode::Up => self.writer.write_all(&[0x1B, 0x5B, 0x41]),
            KeyCode::Down => self.writer.write_all(&[0x1B, 0x5B, 0x42]),
            KeyCode::Right => self.writer.write_all(&[0x1B, 0x5B, 0x43]),
            KeyCode::Left => self.writer.write_all(&[0x1B, 0x5B, 0x44]),
            KeyCode::End => self.writer.write_all(&[0x1B, 0x5B, 0x46]),
            KeyCode::Home => self.writer.write_all(&[0x1B, 0x5B, 0x48]),
            _ => Ok(()),
        }
    }

    pub fn paste(&mut self, clip: String) -> std::io::Result<()> {
        self.writer.write_all(clip.as_bytes())
    }

    pub fn fast_render(&mut self, backend: &mut Backend) {
        let Ok(Some(screen)) = self.output.try_lock().map(|mut lock| lock.new_screen()) else {
            return;
        };
        self.full_render(screen, backend);
    }

    pub fn render(&mut self, backend: &mut Backend) {
        let screen = match self.output.lock() {
            Ok(mut lock) => lock.screen(),
            Err(error) => {
                let mut lock = error.into_inner();
                lock.screen()
            }
        };
        self.full_render(screen, backend);
    }

    fn full_render(&mut self, screen: Screen, backend: &mut Backend) {
        let reset_style = backend.get_style();
        backend.reset_style();
        self.rect.clear(backend);
        let mut text = screen.rows_formatted(0, self.rect.width as u16);
        for line in self.rect.into_iter() {
            if let Some(text) = text.next() {
                backend.go_to(line.row, line.col);
                _ = backend.write_all(&text);
            };
        }
        backend.set_style(reset_style);
        self.cursor.apply(&screen, backend);
    }

    pub fn is_finished(&mut self) -> bool {
        !matches!(self.child.try_wait(), Ok(None))
    }

    pub fn resize(&mut self, rect: Rect) -> Result<(), String> {
        if rect == self.rect {
            return Ok(());
        }
        self.cursor.resize(rect);
        self.pair.master.resize(rect.into()).map_err(|e| e.to_string())
    }
}

struct CursorState {
    hidden: bool,
    row: u16,
    col: u16,
}

impl CursorState {
    fn apply(&mut self, screen: &Screen, backend: &mut Backend) {
        if screen.hide_cursor() {
            if self.hidden {
                return;
            }
            self.hidden = true;
            Backend::hide_cursor();
        } else {
            if !self.hidden {
                return;
            }
            let (row, col) = screen.cursor_position();
            backend.go_to(self.row + row, self.col + col);
            Backend::show_cursor();
        }
    }

    fn resize(&mut self, rect: Rect) {
        self.row = rect.row;
        self.col = rect.col;
    }
}

impl Drop for PtyShell {
    fn drop(&mut self) {
        self.output_handler.abort();
        _ = self.child.kill();
        Backend::hide_cursor();
    }
}

impl From<Rect> for PtySize {
    fn from(rect: Rect) -> Self {
        let rows = rect.height;
        let cols = rect.width as u16;
        PtySize { rows, cols, ..Default::default() }
    }
}

impl From<Rect> for CursorState {
    fn from(rect: Rect) -> Self {
        Self { row: rect.row, col: rect.col, hidden: true }
    }
}
