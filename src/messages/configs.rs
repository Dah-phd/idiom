#[derive(Debug, Clone)]
pub struct Configs {
    indent: String,
}

impl Default for Configs {
    fn default() -> Self {
        Self {
            indent: "    ".to_owned(),
        }
    }
}

impl Configs {
    pub fn get_indent(&self) -> &str {
        return &self.indent;
    }
}