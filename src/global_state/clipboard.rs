use copypasta::{ClipboardContext, ClipboardProvider};
use std::error::Error;

pub struct Clipboard {
    sys: Result<ClipboardContext, Box<dyn Error + Send + Sync + 'static>>,
    tmp: Option<String>,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self { sys: ClipboardContext::new(), tmp: None }
    }
}

impl Clipboard {
    pub fn pull(&mut self) -> Option<String> {
        self.sys.as_mut().ok().and_then(|cc| cc.get_contents().ok()).or(self.tmp.clone())
    }

    pub fn push(&mut self, clip: String) {
        if let Ok(ctx) = self.sys.as_mut() {
            _ = ctx.set_contents(clip);
        } else {
            self.tmp.replace(clip);
        };
    }
}
