use crate::footer::Footer;

#[derive(Debug, Clone)]
pub enum FooterEvent {
    Message(String),
    Error(String),
    Success(String),
}

impl FooterEvent {
    pub fn map(self, footer: &mut Footer) {
        match self {
            Self::Message(message) => footer.message(message),
            Self::Error(message) => footer.error(message),
            Self::Success(message) => footer.success(message),
        }
    }
}
