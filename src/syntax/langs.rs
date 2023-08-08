use crate::configs::FileType;

#[derive(Debug)]
pub struct Lang {
    pub key_words: Vec<&'static str>,
    pub frow_control: Vec<&'static str>,
    pub mod_import: Vec<&'static str>,
}

impl Default for Lang {
    fn default() -> Self {
        Self {
            key_words: vec!["pub", "fn", "struct", "use", "mod", "let", "self", "mut", "crate"],
            frow_control: vec!["if", "loop", "for", "while", "break", "continue"],
            mod_import: vec!["mod", "use", "pub mod", "pub use"],
        }
    }
}

impl From<&FileType> for Lang {
    fn from(value: &FileType) -> Self {
        match value {
            FileType::Python => Self {
                key_words: vec!["def", "class"],
                frow_control: vec![
                    "if", "else", "elif", "for", "while", "break", "continue", "try", "except", "raise",
                ],
                mod_import: vec!["import", "from"],
            },
            _ => Self::default(),
        }
    }
}
