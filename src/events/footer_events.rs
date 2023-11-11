use crate::components::Footer;

pub enum FooterEvent {
    Message(String),
    Overwrite(String),
}

impl FooterEvent {
    pub fn map(self, footer: &mut Footer) {
        match self {
            Self::Message(message) => footer.message(message),
            Self::Overwrite(message) => footer.overwrite(message),
        }
    }
}
