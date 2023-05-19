use std::path::PathBuf;

pub enum Mode {
    Select,
    Insert,
    Popup
}

impl Default for Mode {
    fn default() -> Self {
        Self::Select
    }
}

#[derive(Debug)]
pub enum FileType {
    Rust,
    Python,
    JavaScript,
    Html,
    Yml,
    Toml,
    Unknown
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
                    _ => Self::Unknown
                }
            };
        };
        Self::Unknown
    }

}
