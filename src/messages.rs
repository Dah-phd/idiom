use std::path::PathBuf;

pub enum Mode {
    Select,
    Insert,
    Popup,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Select
    }
}

pub enum Command {
    Exit,
    None,
}

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub enum FileType {
    Rust,
    Python,
    JavaScript,
    Html,
    Yml,
    Toml,
    Unknown,
}

impl FileType {
    pub fn derive_type(path: &PathBuf) -> Self {
        if let Some(extension_os_str) = path.extension() {
            if let Some(extension) = extension_os_str.to_str() {
                return match extension.to_lowercase().as_str() {
                    "rs" => Self::Rust,
                    "py" | "pyw" => Self::Python,
                    "js" => Self::JavaScript,
                    "yml" | "yaml" => Self::Yml,
                    "toml" => Self::Toml,
                    "html" => Self::Html,
                    _ => Self::Unknown,
                };
            };
        };
        Self::Unknown
    }
}

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
