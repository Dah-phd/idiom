mod cursor;
mod tracked_parser;

use crate::{
    error::{IdiomError, IdiomResult},
    global_state::Clipboard,
    render::{
        backend::{Backend, BackendProtocol},
        layout::Rect,
    },
    workspace::CursorPosition,
};
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    style::ContentStyle,
};
use cursor::{CursorState, Position, Select};
use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;
use tracked_parser::{get_ctrl_char, TrackedParser};
use vt100::Screen;

use super::backend::StyleExt;

/// Run another tui app within the context of idiom
pub struct PtyShell {
    pair: PtyPair,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output_handler: JoinHandle<std::io::Result<()>>,
    output: Arc<Mutex<TrackedParser>>,
    rect: Rect,
    cursor: CursorState,
    select: Select,
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

        Ok(Self {
            rect,
            pair,
            child,
            writer,
            output,
            output_handler,
            cursor: CursorState::from(rect),
            select: Select::default(),
        })
    }

    pub fn map_key(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> std::io::Result<()> {
        if let KeyEvent {
            code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT, ..
        } = key
        {
            if let Some(clip) = self.copy() {
                clipboard.push(clip);
            }
            return Ok(());
        }

        self.select.clear();

        if let Some(ctrl_char) = get_ctrl_char(key) {
            return self.writer.write_all(&[ctrl_char]);
        }

        match key.code {
            KeyCode::Esc => self.writer.write_all(&[0x1B]),
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

    pub fn map_mouse(&mut self, event: MouseEvent) {
        match event {
            MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, .. } => {
                let Some((row, col)) = self.rect.raw_relative_position(row, column) else {
                    self.select.clear();
                    return;
                };
                self.select.mouse_down(row, col);
            }
            MouseEvent { kind: MouseEventKind::Drag(MouseButton::Left), column, row, .. } => {
                let Some((row, col)) = self.rect.raw_relative_position(row, column) else {
                    self.select.clear();
                    return;
                };
                self.select.mouse_drag(row, col);
            }
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => {
                let Some((row, col)) = self.rect.raw_relative_position(row, column) else {
                    self.select.clear();
                    return;
                };
                self.select.mouse_up(row, col);
            }
            _ => (),
        }
    }

    pub fn paste(&mut self, clip: String) -> std::io::Result<()> {
        self.select.clear();
        self.writer.write_all(clip.as_bytes())
    }

    pub fn copy(&self) -> Option<String> {
        let screen = match self.output.lock() {
            Ok(mut lock) => lock.screen(),
            Err(error) => {
                let mut lock = error.into_inner();
                lock.screen()
            }
        };
        let (from, to) = self.select.get()?;
        let clip = screen.contents_between(from.row, from.col, to.row, to.col);
        Some(clip)
    }

    pub fn fast_render(&mut self, backend: &mut Backend) {
        if self.select.collect_update() {
            return self.render(backend);
        }

        let Ok(Some(screen)) = self.output.try_lock().map(|mut lock| lock.new_screen()) else {
            return;
        };
        match self.select.get() {
            Some(select) => self.render_with_select(screen, select, backend),
            None => self.render_no_select(screen, backend),
        };
    }

    pub fn render(&mut self, backend: &mut Backend) {
        let screen = match self.output.lock() {
            Ok(mut lock) => lock.screen(),
            Err(error) => {
                let mut lock = error.into_inner();
                lock.screen()
            }
        };
        match self.select.get() {
            Some(select) => self.render_with_select(screen, select, backend),
            None => self.render_no_select(screen, backend),
        };
    }

    fn render_no_select(&mut self, screen: Screen, backend: &mut Backend) {
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

    fn render_with_select(&mut self, screen: Screen, (from, to): (Position, Position), backend: &mut Backend) {
        let reset_style = backend.get_style();
        backend.reset_style();
        self.rect.clear(backend);
        let mut text = screen.rows_formatted(0, self.rect.width as u16).enumerate();
        let select_text = screen.contents_between(from.row, from.col, to.row, to.col);
        let mut select_lines = select_text.lines();
        let start = CursorPosition::from(from);
        let end = CursorPosition::from(to);
        for line in self.rect.into_iter() {
            if let Some((index, text)) = text.next() {
                if index < start.line || index > end.line {
                    backend.go_to(line.row, line.col);
                    _ = backend.write_all(&text);
                    continue;
                }
                if let Some(raw_text) = select_lines.next() {
                    if start.line == index {
                        backend.go_to(line.row, line.col);
                        for cell_col in 0..from.col {
                            if let Some(cell) = screen.cell(from.row, cell_col) {
                                backend.print(cell.contents());
                            } else {
                                backend.print(' ');
                            };
                        }
                        backend.print_styled(raw_text, ContentStyle::reversed());
                        continue;
                    }
                    if end.line == index {
                        backend.go_to(line.row, line.col);
                        backend.print_styled(raw_text, ContentStyle::reversed());
                        let mut cell_col = to.col;
                        while let Some(cell) = screen.cell(to.row, cell_col) {
                            cell_col += 1;
                            backend.print(cell.contents());
                        }
                        continue;
                    }
                    line.render_styled(raw_text, ContentStyle::reversed(), backend);
                }
            };
        }
        backend.set_style(reset_style);
        self.cursor.apply(&screen, backend);
    }

    pub fn is_finished(&mut self) -> bool {
        !matches!(self.child.try_wait(), Ok(None))
    }

    pub fn resize(&mut self, rect: Rect) -> Result<(), String> {
        self.select.clear();
        if rect == self.rect {
            return Ok(());
        }
        self.cursor.resize(rect);
        self.pair.master.resize(rect.into()).map_err(|e| e.to_string())
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
