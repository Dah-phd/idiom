use crate::{
    error::{IdiomError, IdiomResult},
    global_state::{Clipboard, GlobalState, PopupMessage},
    render::{
        backend::{Backend, BackendProtocol},
        layout::Rect,
    },
};
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use fuzzy_matcher::skim::SkimMatcherV2;
use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize};
use std::{
    io::{Read, Write},
    sync::{Arc, RwLock},
};
use tokio::task::JoinHandle;
use vt100::{Parser, Screen};

/// Run another tui app within the context of idiom
pub struct PopupApplet {
    pair: PtyPair,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output_handler: JoinHandle<std::io::Result<()>>,
    output: Arc<RwLock<(Parser, bool)>>,
    rect: Rect,
}

impl PopupApplet {
    pub fn default_cmd(rect: Rect) -> IdiomResult<Self> {
        Self::new(CommandBuilder::new_default_prog(), rect)
    }

    pub fn run(cmd: &str, rect: Rect) -> IdiomResult<Self> {
        Self::new(CommandBuilder::new(cmd), rect)
    }

    pub fn new(mut cmd: CommandBuilder, rect: Rect) -> IdiomResult<Self> {
        let rows = rect.height;
        let cols = rect.width as u16;
        let system = native_pty_system();
        let size = PtySize { rows, cols, ..Default::default() };
        let pair = system.openpty(size).map_err(|err| IdiomError::any(err))?;

        cmd.cwd("./");
        let child = pair.slave.spawn_command(cmd).map_err(|error| IdiomError::any(error))?;
        let writer = pair.master.take_writer().map_err(|error| IdiomError::any(error))?;
        let mut reader = pair.master.try_clone_reader().map_err(|error| IdiomError::any(error))?;
        let output = Arc::new(RwLock::new((Parser::new(rows, cols, 0), false)));
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
                let mut lock = output_writer.write().unwrap();
                lock.0.process(&processed_buf);
                lock.1 = true;
                processed_buf.clear();
            }
        });

        Ok(Self { rect, pair, child, writer, output, output_handler })
    }

    fn full_render(&self, screen: Screen, backend: &mut Backend) {
        self.rect.clear(backend);
        let mut screen = screen.rows_formatted(0, self.rect.width as u16);
        for line in self.rect.into_iter() {
            if let Some(text) = screen.next() {
                backend.go_to(line.row, line.col);
                _ = backend.write_all(&text);
            } else {
                line.render_empty(backend);
            };
        }
    }

    pub fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, matcher: &SkimMatcherV2) -> PopupMessage {
        match key.code {
            KeyCode::Char('q') => return PopupMessage::Clear,
            KeyCode::Char(input) => {
                _ = self.writer.write_all(&[input as u8]).unwrap();
            }
            KeyCode::Backspace => {
                _ = self.writer.write_all(&[8]);
            }
            KeyCode::Enter => _ = self.writer.write_all(&[b'\n']),
            KeyCode::Left => {
                _ = self.writer.write_all(&[27, 91, 68]);
            }
            KeyCode::Right => {
                _ = self.writer.write_all(&[27, 91, 67]);
            }
            KeyCode::Up => {
                _ = self.writer.write_all(&[27, 91, 65]);
            }
            KeyCode::Down => {
                _ = self.writer.write_all(&[27, 91, 66]);
            }
            // KeyCode::Home => todo!(),
            // KeyCode::End => todo!(),
            // KeyCode::PageUp => todo!(),
            // KeyCode::PageDown => todo!(),
            KeyCode::Tab => _ = self.writer.write_all(&[b'\t']),
            // KeyCode::BackTab => todo!(),
            // KeyCode::Delete => todo!(),
            // KeyCode::Insert => todo!(),
            // KeyCode::F(_) => todo!(),
            // KeyCode::Null => todo!(),
            // KeyCode::Esc => todo!(),
            // KeyCode::CapsLock => todo!(),
            // KeyCode::ScrollLock => todo!(),
            // KeyCode::NumLock => todo!(),
            // KeyCode::PrintScreen => todo!(),
            // KeyCode::Pause => todo!(),
            // KeyCode::Menu => todo!(),
            // KeyCode::KeypadBegin => todo!(),
            // KeyCode::Media(_) => todo!(),
            // KeyCode::Modifier(_) => todo!(),
            _ => (),
        }
        PopupMessage::None
    }

    fn mouse_map(&mut self, _event: MouseEvent) -> PopupMessage {
        todo!()
    }

    fn paste_passthrough(&mut self, _clip: String, _matcher: &SkimMatcherV2) -> PopupMessage {
        todo!();
    }

    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        let screen = if let Ok(mut lock) = self.output.try_write() {
            if !lock.1 {
                return;
            }
            lock.1 = false;
            let size = PtySize { rows: self.rect.height, cols: self.rect.width as u16, ..Default::default() };
            _ = self.pair.master.resize(size);
            lock.0.screen().clone()
        } else {
            return;
        };
        self.full_render(screen, gs.backend());
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        if let Ok(screen) = self.output.read().map(|lock| lock.0.screen().clone()) {
            let size = PtySize { rows: self.rect.height, cols: self.rect.width as u16, ..Default::default() };
            _ = self.pair.master.resize(size);
            self.full_render(screen, gs.backend());
        }
    }

    fn collect_update_status(&mut self) -> bool {
        true
    }
    fn mark_as_updated(&mut self) {}
}

impl Drop for PopupApplet {
    fn drop(&mut self) {
        self.output_handler.abort();
        _ = self.child.kill();
    }
}
