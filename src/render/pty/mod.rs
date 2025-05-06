mod cursor;
mod parser;

use crate::{
    error::{IdiomError, IdiomResult},
    global_state::GlobalState,
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
use parser::{get_ctrl_char, parse_cell_style, TrackedParser};
use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize};
use std::io::{Read, Write};
use tokio::task::JoinHandle;

use super::backend::StyleExt;

const CONTROLS_HELP: &str = "Term Overlay: MouseLeft drag select / MouseRight copy select";

/// Run another tui app within the context of idiom
pub struct PtyShell {
    pair: PtyPair,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    process_handle: JoinHandle<std::io::Result<()>>,
    parser: TrackedParser,
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
        let parser = TrackedParser::new(size.rows, size.cols);
        let buffer = parser.buffer_access();

        let process_handle = tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            loop {
                let size = reader.read(&mut buf)?;
                if size == 0 {
                    return Ok(());
                }
                let mut lock = buffer.lock().expect("lock on PtyShell read");
                lock.extend_from_slice(&buf[..size]);
            }
        });

        Ok(Self {
            rect,
            pair,
            child,
            writer,
            parser,
            process_handle,
            cursor: CursorState::from(rect),
            select: Select::default(),
        })
    }

    pub fn map_key(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> std::io::Result<()> {
        self.select.clear();

        if key.modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT {
            match key.code {
                KeyCode::Down => {
                    self.parser.scroll_down();
                    self.inner_render(gs.backend());
                    return Ok(());
                }
                KeyCode::Up => {
                    self.parser.scroll_up();
                    self.inner_render(gs.backend());
                    return Ok(());
                }
                _ => {}
            }
        }

        self.parser.scroll_to_end();

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

    pub fn map_mouse(&mut self, event: MouseEvent, gs: &mut GlobalState) {
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
            MouseEvent { kind: MouseEventKind::Down(MouseButton::Right), column, row, .. } => {
                let Some((row, col)) = self.rect.raw_relative_position(row, column) else {
                    return;
                };
                let position = Position { row, col };
                let Some((start, end)) = self.select.get() else { return };
                if position < start || position > end {
                    return;
                };
                if let Some(clip) = self.select.copy_clip(self.parser.screen()) {
                    gs.success("Select from embeded copied!");
                    gs.clipboard.push(clip);
                };
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, column, row, .. } => {
                if self.rect.raw_relative_position(row, column).is_none() {
                    return;
                };
                self.select.clear();
                self.parser.scroll_up();
                self.inner_render(gs.backend());
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, column, row, .. } => {
                if self.rect.raw_relative_position(row, column).is_none() {
                    return;
                };
                self.select.clear();
                self.parser.scroll_down();
                self.inner_render(gs.backend());
            }
            _ => (),
        }
    }

    pub fn paste(&mut self, clip: String) -> std::io::Result<()> {
        self.select.clear();
        self.writer.write_all(clip.as_bytes())
    }

    pub fn fast_render(&mut self, backend: &mut Backend) {
        if self.select.collect_update() {
            return self.render(backend);
        }

        if !self.parser.try_parse() {
            return;
        }
        self.inner_render(backend);
    }

    pub fn render(&mut self, backend: &mut Backend) {
        _ = self.parser.try_parse();
        self.inner_render(backend);
    }

    fn inner_render(&mut self, backend: &mut Backend) {
        match self.select.get() {
            Some(select) => self.render_with_select(select, backend),
            None => self.render_no_select(backend),
        };
    }

    fn render_no_select(&mut self, backend: &mut Backend) {
        let screen = self.parser.screen();
        let reset_style = backend.get_style();
        backend.reset_style();
        self.rect.clear(backend);
        {
            let mut text = screen.rows_formatted(0, self.rect.width as u16);
            for line in self.rect.into_iter() {
                if let Some(text) = text.next() {
                    backend.go_to(line.row, line.col);
                    _ = backend.write_all(&text);
                };
            }
        }
        backend.set_style(reset_style);
        self.cursor.apply(screen, backend);
    }

    fn render_with_select(&mut self, (from, to): (Position, Position), backend: &mut Backend) {
        let screen = self.parser.screen();
        let reset_style = backend.get_style();
        backend.reset_style();
        self.rect.clear(backend);
        {
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
                        backend.reset_style();
                        backend.go_to(line.row, line.col);

                        if start.line == index {
                            for cell_col in 0..from.col {
                                if let Some(cell) = screen.cell(from.row, cell_col) {
                                    let style = parse_cell_style(cell);
                                    backend.print_styled(cell.contents(), style);
                                };
                            }
                        }

                        backend.print_styled(raw_text, ContentStyle::reversed());

                        if end.line == index {
                            let mut cell_col = to.col;
                            while let Some(cell) = screen.cell(to.row, cell_col) {
                                cell_col += 1;
                                let style = parse_cell_style(cell);
                                backend.print_styled(cell.contents(), style);
                            }
                        }
                    }
                };
            }
        }
        backend.set_style(reset_style);
        self.cursor.apply(screen, backend);
    }

    pub fn is_finished(&mut self) -> bool {
        !matches!(self.child.try_wait(), Ok(None))
    }

    pub fn resize(&mut self, rect: Rect) -> Result<(), String> {
        self.select.clear();
        if rect == self.rect {
            return Ok(());
        }

        self.rect = rect;
        self.cursor.resize(rect);

        let size = PtySize::from(rect);
        self.parser.resize(size.rows, size.cols);
        self.pair.master.resize(size).map_err(|e| e.to_string())
    }

    pub fn controls_help(gs: &mut GlobalState) {
        gs.message(CONTROLS_HELP);
        gs.message(CONTROLS_HELP);
    }
}

impl Drop for PtyShell {
    fn drop(&mut self) {
        self.process_handle.abort();
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
