use crate::{error::{IdiomError, IdiomResult}, global_state::{Clipboard, GlobalState, PopupMessage}, render::layout::Rect};
use crossterm::event::{KeyEvent, KeyCode, MouseEvent};
use fuzzy_matcher::skim::SkimMatcherV2;
use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex, RwLock},
};
use tokio::task::JoinHandle;
use super::PopupInterface;

/// Run another tui app within the context of idiom
pub struct PopupApplet {
    pair: PtyPair,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output_handler: JoinHandle<std::io::Result<()>>,
    output: Arc<RwLock<String>>,
    rect: Rect,
    dd: Vec<String>,
}

impl PopupApplet {
    pub fn build(rect: Rect) -> IdiomResult<Box<Self>> {
        let width = rect.width - 10;
        let height = rect.height - 4;
        let rect = rect.center(height, width);
        Ok(Box::new(Self::new(rect)?))
    }

    fn new(rect: Rect) -> IdiomResult<Self> {
        let rows = rect.height;
        let cols = rect.width as u16;
        let system = native_pty_system();
        let pair = system
            .openpty(PtySize { rows, cols, ..Default::default() })
            .map_err(|err| IdiomError::any(err))?;

        let mut cmd = CommandBuilder::new("gitui");
        cmd.cwd("./");
        let child = pair.slave.spawn_command(cmd).map_err(|error| IdiomError::any(error))?;
        let writer = pair.master.take_writer().map_err(|error| IdiomError::any(error))?;
        let mut reader = pair.master.try_clone_reader().map_err(|error| IdiomError::any(error))?;
        let output: Arc<RwLock<String>> = Arc::default();
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
                match std::str::from_utf8(&processed_buf) {
                    Ok(data) => {
                        let mut output = output_writer.write().unwrap();
                        output.push_str(data);
                        processed_buf.clear();
                    }
                    Err(..) => ()
                }
            }
        });

        Ok(
            Self {
                rect,
                pair,
                child,
                writer,
                output,
                dd: Vec::new(),
                output_handler,
            },
        )
    }
}

impl PopupInterface for PopupApplet {
    fn collect_update_status(&mut self) -> bool {
        true
    }

    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, matcher: &SkimMatcherV2) -> PopupMessage {
        crate::global_state::PopupMessage::None        
    }

    fn map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, matcher: &SkimMatcherV2) -> PopupMessage {
        match key.code {
            KeyCode::Char('q') => return PopupMessage::Clear,
            KeyCode::Char(input) => {
                _ = self.writer.write_all(&[input as u8]).unwrap();
            
            }
            KeyCode::Backspace => {
                _ = self.writer.write_all(&[8]);
            }
            KeyCode::Enter => {_ = self.writer.write_all(&[b'\n'])},
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
            // KeyCode::Tab => todo!(),
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
            _ => ()
        }
        PopupMessage::Clear
    }

    fn mark_as_updated(&mut self) {
        
    }

    fn mouse_map(&mut self, _event: MouseEvent) -> PopupMessage {
        todo!()
    }

    fn paste_passthrough(&mut self, _clip: String, _matcher: &SkimMatcherV2) -> PopupMessage {
        todo!();
    }

    fn render(&mut self, gs: &mut GlobalState) {
        let lines = self.rect.into_iter().collect::<Vec<_>>();
        let Ok(text) = self.output.try_read().map(|x| x.clone()) else {return};
        let rev_text = text.lines().rev();
        let rev_lines = lines.into_iter().rev();
        for (line, text) in rev_lines.zip(rev_text) {
            line.render(text, gs.backend());
        }
    }
}

