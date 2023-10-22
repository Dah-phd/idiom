use super::utils::insert_clip;
use cli_clipboard::{ClipboardContext, ClipboardProvider};
use std::fmt::Debug;

use super::CursorPosition;

pub struct Clipboard {
    ctx: ClipboardContext,
}

impl Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{{ Clipboard Object }}")
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self { ctx: ClipboardContext::new().unwrap() }
    }
}

impl Clipboard {
    pub fn get(&mut self) -> Option<String> {
        self.ctx.get_contents().ok()
    }

    pub fn push(&mut self, clip: String) -> Option<()> {
        self.ctx.set_contents(clip).ok()
    }

    pub fn copy(&mut self, content: &mut [String], from: &CursorPosition, to: &CursorPosition) {
        if from.line == to.line {
            self.push(content[from.line][from.char..to.char].to_owned());
        } else {
            let mut at_line = from.line;
            let mut clip_vec = Vec::new();
            clip_vec.push(content[from.line][from.char..].to_owned());
            while at_line < to.line {
                at_line += 1;
                if at_line != to.line {
                    clip_vec.push(content[at_line].to_owned())
                } else {
                    clip_vec.push(content[at_line][..to.char].to_owned())
                }
            }
            self.push(clip_vec.join("\n"));
        }
    }

    pub fn copy_line(&mut self, content: &mut [String], cursor: &CursorPosition) {
        let mut line = content[cursor.line].to_owned();
        line.push('\n');
        self.push(line);
    }

    pub fn paste(&mut self, content: &mut Vec<String>, cursor: CursorPosition) -> Option<CursorPosition> {
        Some(insert_clip(self.get()?, content, cursor))
    }
}
