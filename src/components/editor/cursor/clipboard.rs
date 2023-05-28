use std::fmt::Debug;

use cli_clipboard::{ClipboardContext, ClipboardProvider};

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
        Self {
            ctx: ClipboardContext::new().unwrap(),
        }
    }
}

impl Clipboard {
    pub fn get(&mut self) -> Option<String> {
        self.ctx.get_contents().ok()
    }

    pub fn push(&mut self, content: String) -> Option<()> {
        self.ctx.set_contents(content).ok()
    }
}
