use crate::configs::FileType;

#[derive(Debug, Clone)]
pub struct Lang {
    pub file_type: FileType,
    pub declaration: Vec<&'static str>,
    pub key_words: Vec<&'static str>,
    pub frow_control: Vec<&'static str>,
    pub mod_import: Vec<&'static str>,
}

impl Lang {
    pub fn is_keyword(&self, token: &str) -> bool {
        self.declaration.contains(&token) || self.key_words.contains(&token)
    }

    pub fn completelable(&self, line: &str, idx: usize) -> bool {
        let mut last_char = ' ';
        let mut curr_token = String::new();
        let mut prev_token = String::new();
        for (char_idx, ch) in line.char_indices() {
            if ch.is_alphabetic() || ch == '_' {
                if char_idx + 1 == idx {
                    return " (.".contains(last_char) && !self.declaration.contains(&prev_token.as_str());
                } else {
                    curr_token.push(ch);
                }
            } else {
                prev_token.extend(curr_token.drain(..));
            }
            last_char = ch;
        }
        false
    }
}

impl From<FileType> for Lang {
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Rust => Self {
                file_type,
                declaration: vec!["fn", "struct", "enum", "type", "const"],
                key_words: vec![
                    "pub", "use", "mod", "let", "self", "mut", "crate", "async", "super", "impl", "Self",
                ],
                frow_control: vec![
                    "if", "loop", "for", "in", "while", "break", "continue", "await", "return", "match", "else",
                ],
                mod_import: vec!["mod", "use", "pub mod", "pub use"],
            },
            FileType::Python => Self {
                file_type,
                declaration: vec!["def", "class"],
                key_words: vec![],
                frow_control: vec![
                    "if", "else", "elif", "for", "while", "break", "continue", "try", "except", "raise",
                ],
                mod_import: vec!["import", "from"],
            },
            FileType::MarkDown => {
                Self { file_type, declaration: vec![], key_words: vec![], frow_control: vec![], mod_import: vec![] }
            }
            _ => Self {
                file_type,
                declaration: vec![
                    "fn", "struct", "enum", "type", "const", "def", "class", "var", "function",
                ],
                key_words: vec![
                    "pub", "use", "mod", "let", "self", "mut", "crate", "async", "super", "impl", "Self",
                ],
                frow_control: vec![
                    "if", "loop", "for", "in", "while", "break", "continue", "await", "return", "match", "else",
                ],
                mod_import: vec!["mod", "use", "pub mod", "pub use", "from", "import"],
            },
        }
    }
}
