use crate::configs::FileType;

#[derive(Debug, Clone)]
pub struct Lang {
    pub file_type: FileType,
    pub comment_start: Vec<&'static str>,
    pub declaration: Vec<&'static str>,
    pub key_words: Vec<&'static str>,
    pub frow_control: Vec<&'static str>,
    pub mod_import: Vec<&'static str>,
}

impl Lang {
    pub fn is_keyword(&self, token: &str) -> bool {
        self.declaration.contains(&token) || self.key_words.contains(&token)
    }

    pub fn is_import(&self, token: &str) -> bool {
        self.mod_import.contains(&token)
    }

    pub fn is_comment(&self, line: &str) -> bool {
        for start in self.comment_start.iter() {
            if line.trim_start().starts_with(start) {
                return true;
            }
        }
        false
    }

    pub fn completelable(&self, line: &str, idx: usize) -> bool {
        let mut curr_token = String::new();
        let mut prev_token = String::new();
        let mut trigger = false;
        for (char_idx, ch) in line.char_indices() {
            if ch.is_alphabetic() || ch == '_' {
                if char_idx + 1 == idx {
                    return trigger
                        || prev_token.is_empty()
                            && curr_token.len() < 4
                            && !self.declaration.contains(&prev_token.as_str());
                }
                curr_token.push(ch);
            } else {
                if " (.".contains(ch) {
                    trigger = true;
                }
                prev_token = std::mem::take(&mut curr_token);
            }
        }
        false
    }
}

impl From<FileType> for Lang {
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Rust => Self {
                file_type,
                comment_start: vec!["//", "///"],
                declaration: vec!["fn", "struct", "enum", "type", "const"],
                key_words: vec![
                    "pub", "use", "mod", "let", "self", "mut", "crate", "async", "super", "impl", "Self",
                ],
                frow_control: vec![
                    "if", "loop", "for", "in", "while", "break", "continue", "await", "return", "match", "else",
                ],
                mod_import: vec!["mod", "use"],
            },
            FileType::Python => Self {
                comment_start: vec!["#"],
                file_type,
                declaration: vec!["def", "class"],
                key_words: vec![],
                frow_control: vec![
                    "if", "else", "elif", "for", "while", "break", "continue", "try", "except", "raise", "with",
                ],
                mod_import: vec!["import", "from", "as"],
            },
            FileType::MarkDown => Self {
                file_type,
                comment_start: vec![],
                declaration: vec![],
                key_words: vec![],
                frow_control: vec![],
                mod_import: vec![],
            },
            _ => Self {
                file_type,
                comment_start: vec!["#", "//"],
                declaration: vec![
                    "fn", "struct", "enum", "type", "const", "def", "class", "var", "function",
                ],
                key_words: vec![
                    "pub", "use", "mod", "let", "self", "mut", "crate", "async", "super", "impl", "Self",
                ],
                frow_control: vec![
                    "if", "loop", "for", "in", "while", "break", "continue", "await", "return", "match", "else",
                ],
                mod_import: vec!["mod", "use", "from", "import"],
            },
        }
    }
}
