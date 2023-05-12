use std::path::PathBuf;

pub enum Mode {
    Select,
    Insert,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Select
    }
}

pub enum FileType {
    Rust,
    Python,
    JavaScript,
    Html,
    Yaml,
    Toml,
    Unknown
}
// impl FileType {
//     pub fn derive_type(path: &PathBuf) -> Self {
//         if let Some(full_path) = path.as_os_str().to_str() {
//             if 
//         }
//         Self::Unknown
//     }

// }
