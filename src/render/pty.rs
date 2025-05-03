use crate::{
    error::{IdiomError, IdiomResult},
    render::{
        backend::{Backend, BackendProtocol},
        layout::Rect,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
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

    pub fn map_key(&mut self, key: &KeyEvent) -> std::io::Result<()> {
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

    pub fn map_mouse(&mut self, event: MouseEvent) -> std::io::Result<()> {
        let event_buffer = match event.kind {
            MouseEventKind::Down(MouseButton::Left) => mouse_down(event.row, event.column),
            MouseEventKind::Drag(MouseButton::Left) => mouse_drag(event.row, event.column),
            MouseEventKind::Up(MouseButton::Left) => mouse_up(event.row, event.column),
            _ => return Ok(()),
        };
        if let Some(buf) = event_buffer {
            return self.writer.write_all(&buf);
        }
        Ok(())
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
        // screen.contents_between(start_row, start_col, end_row, end_col)
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

fn get_ctrl_char(key: &KeyEvent) -> Option<u8> {
    if let KeyEvent { code: KeyCode::Char(ch), modifiers: KeyModifiers::CONTROL, .. } = key {
        let ctrl_char = match ch {
            '@' => 0x0,
            'a' => 0x1,
            'b' => 0x2,
            'c' => 0x3,
            'd' => 0x4,
            'e' => 0x5,
            'f' => 0x6,
            'g' => 0x7,
            'h' => 0x8,
            'i' => 0x9,
            'j' => 0x10,
            'k' => 0x11,
            'l' => 0x12,
            'm' => 0x13,
            'n' => 0x14,
            'o' => 0x15,
            'p' => 0x16,
            'q' => 0x17,
            'r' => 0x18,
            's' => 0x19,
            't' => 0x20,
            'u' => 0x21,
            'v' => 0x22,
            'w' => 0x23,
            'x' => 0x24,
            'y' => 0x25,
            'z' => 0x26,
            '[' => 0x27,
            '\\' => 0x28,
            ']' => 0x29,
            '^' => 0x30,
            '_' => 0x30,
            _ => return None,
        };
        return Some(ctrl_char);
    };
    None
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

fn mouse_drag(row: u16, col: u16) -> Option<[u8; 6]> {
    let row = u8::try_from(row.checked_add(33)?).ok()?;
    let col = u8::try_from(col.checked_add(33)?).ok()?;
    Some([27, 91, 77, 64, col, row])
}

fn mouse_down(row: u16, col: u16) -> Option<[u8; 6]> {
    let row = u8::try_from(row.checked_add(33)?).ok()?;
    let col = u8::try_from(col.checked_add(33)?).ok()?;
    Some([27, 91, 77, 32, col, row])
}

fn mouse_up(row: u16, col: u16) -> Option<[u8; 6]> {
    let row = u8::try_from(row.checked_add(33)?).ok()?;
    let col = u8::try_from(col.checked_add(33)?).ok()?;
    Some([27, 91, 77, 35, col, row])
}

#[cfg(test)]
mod test {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    use super::{mouse_down, mouse_drag, mouse_up};

    fn parse_cb(cb: u8) -> Option<(MouseEventKind, KeyModifiers)> {
        let button_number = (cb & 0b0000_0011) | ((cb & 0b1100_0000) >> 4);
        let dragging = cb & 0b0010_0000 == 0b0010_0000;

        let kind = match (button_number, dragging) {
            (0, false) => MouseEventKind::Down(MouseButton::Left),
            (1, false) => MouseEventKind::Down(MouseButton::Middle),
            (2, false) => MouseEventKind::Down(MouseButton::Right),
            (0, true) => MouseEventKind::Drag(MouseButton::Left),
            (1, true) => MouseEventKind::Drag(MouseButton::Middle),
            (2, true) => MouseEventKind::Drag(MouseButton::Right),
            (3, false) => MouseEventKind::Up(MouseButton::Left),
            (3, true) | (4, true) | (5, true) => MouseEventKind::Moved,
            (4, false) => MouseEventKind::ScrollUp,
            (5, false) => MouseEventKind::ScrollDown,
            (6, false) => MouseEventKind::ScrollLeft,
            (7, false) => MouseEventKind::ScrollRight,
            // We do not support other buttons.
            _ => return None,
        };

        let mut modifiers = KeyModifiers::empty();

        if cb & 0b0000_0100 == 0b0000_0100 {
            modifiers |= KeyModifiers::SHIFT;
        }
        if cb & 0b0000_1000 == 0b0000_1000 {
            modifiers |= KeyModifiers::ALT;
        }
        if cb & 0b0001_0000 == 0b0001_0000 {
            modifiers |= KeyModifiers::CONTROL;
        }

        Some((kind, modifiers))
    }

    fn parse_csi_normal_mouse(buffer: &[u8]) -> Option<MouseEvent> {
        // Normal mouse encoding: ESC [ M CB Cx Cy (6 characters only).

        assert!(buffer.starts_with(b"\x1B[M")); // ESC [ M

        if buffer.len() < 6 {
            return None;
        }

        let cb = buffer[3].checked_sub(32)?;
        let (kind, modifiers) = parse_cb(cb)?;

        // See http://www.xfree86.org/current/ctlseqs.html#Mouse%20Tracking
        // The upper left character position on the terminal is denoted as 1,1.
        // Subtract 1 to keep it synced with cursor
        let cx = u16::from(buffer[4].saturating_sub(32)) - 1;
        let cy = u16::from(buffer[5].saturating_sub(32)) - 1;

        Some(MouseEvent { kind, column: cx, row: cy, modifiers })
    }

    #[test]
    fn mouse_left_down() {
        assert_eq!(
            Some(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 0,
                row: 10,
                modifiers: KeyModifiers::empty()
            }),
            mouse_down(10, 0).and_then(|buf| parse_csi_normal_mouse(&buf))
        );
        assert_eq!(
            Some(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 10,
                row: 20,
                modifiers: KeyModifiers::empty()
            }),
            mouse_down(20, 10).and_then(|buf| parse_csi_normal_mouse(&buf))
        );
    }

    #[test]
    fn mouse_left_drag() {
        assert_eq!(
            Some(MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: 0,
                row: 10,
                modifiers: KeyModifiers::empty()
            }),
            mouse_drag(10, 0).and_then(|buf| parse_csi_normal_mouse(&buf))
        );
        assert_eq!(
            Some(MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: 10,
                row: 20,
                modifiers: KeyModifiers::empty()
            }),
            mouse_drag(20, 10).and_then(|buf| parse_csi_normal_mouse(&buf))
        );
    }

    #[test]
    fn mouse_left_up() {
        assert_eq!(
            Some(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 0,
                row: 10,
                modifiers: KeyModifiers::empty()
            }),
            mouse_up(10, 0).and_then(|buf| parse_csi_normal_mouse(&buf))
        );
        assert_eq!(
            Some(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 10,
                row: 20,
                modifiers: KeyModifiers::empty()
            }),
            mouse_up(20, 10).and_then(|buf| parse_csi_normal_mouse(&buf))
        );
    }
}
