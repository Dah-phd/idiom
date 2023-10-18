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

    pub fn paste(&mut self, content: &mut Vec<String>, mut cursor: CursorPosition) -> Option<CursorPosition> {
        if let Some(clip) = self.get() {
            let mut lines: Vec<_> = clip.split('\n').collect();
            if lines.len() == 1 {
                let text = lines[0];
                content[cursor.line].insert_str(cursor.char, lines[0]);
                cursor.char += text.len();
                return Some(cursor);
            } else {
                let line = content.remove(cursor.line);
                let (prefix, suffix) = line.split_at(cursor.char);
                let mut first_line = prefix.to_owned();
                first_line.push_str(lines.remove(0));
                content.insert(cursor.line, first_line);
                let last_idx = lines.len() - 1;
                for (idx, select) in lines.iter().enumerate() {
                    let next_line = if idx == last_idx {
                        let mut last_line = select.to_string();
                        cursor.char = last_line.len();
                        last_line.push_str(suffix);
                        last_line
                    } else {
                        select.to_string()
                    };
                    content.insert(cursor.line + 1, next_line);
                    cursor.line += 1;
                }
                return Some(cursor);
            }
        }
        None
    }
}
