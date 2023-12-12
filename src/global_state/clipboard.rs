use copypasta::{ClipboardContext, ClipboardProvider};

pub enum Clipboard {
    System(ClipboardContext),
    Internal(Vec<String>),
}

impl Default for Clipboard {
    fn default() -> Self {
        if let Ok(clipboard) = ClipboardContext::new() {
            Self::System(clipboard)
        } else {
            Self::Internal(Vec::new())
        }
    }
}

impl Clipboard {
    pub fn pull(&mut self) -> Option<String> {
        match self {
            Self::System(cliboard) => cliboard.get_contents().ok(),
            Self::Internal(inner) => inner.pop(),
        }
    }

    pub fn push(&mut self, clip: String) {
        match self {
            Self::System(clipboard) => {
                let _ = clipboard.set_contents(clip);
            }
            Self::Internal(inner) => {
                inner.push(clip);
            }
        }
    }
}
