use std::path::PathBuf;

#[derive(Debug)]
pub struct File {
    pub content: Vec<String>,
    buffer: String,
    pub path: PathBuf,
}

impl File {
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        Ok(Self {
            content: content.split('\n').map(String::from).collect(),
            buffer: String::new(),
            path,
        })
    }

    pub fn compare(&self) -> Option<Vec<(usize, String)>> {
        let mut deltas = vec![];
        let new_content = std::fs::read_to_string(&self.path)
            .unwrap_or_default()
            .split('\n')
            .map(String::from)
            .collect::<Vec<_>>();
        let max = if self.content.len() > new_content.len() {
            self.content.len()
        } else {
            new_content.len()
        };

        let empty_str = String::from("");
        for idx in 0..max {
            let line = self.content.get(idx).unwrap_or(&empty_str);
            let new_line = new_content.get(idx).unwrap_or(&empty_str);
            if line != new_line {
                deltas.push((idx, format!("\nOLD LINE:\n{}\n NEW LINE:\n{}\n", line, new_line)))
            }
        }

        if deltas.is_empty() {
            return None;
        }
        Some(deltas)
    }
}
