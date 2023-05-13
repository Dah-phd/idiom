use std::path::PathBuf;
use super::linter::linter;


#[derive(Debug)]
pub struct Editor {
    pub content: Vec<String>,
    pub cursor: (usize, usize), // line, char
    pub path: PathBuf,
}

impl Editor {
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        Ok(Self {
            content: content.split('\n').map(String::from).collect(),
            cursor: (0, 0),
            path,
        })
    }

    pub fn push_str(&mut self, c:&str) {
        if let Some(line) = self.content.get_mut(self.cursor.0) {
            line.insert_str(self.cursor.1, c);
            self.cursor.1 += c.len();
        }
    }

    pub fn new_line(&mut self) {
        if let Some(line) = self.content.get(self.cursor.0) {
            if line.len() -1 > self.cursor.1 {
                let (replace_line, new_line) = line.split_at(self.cursor.1);
                let new_line = String::from(new_line);
                self.content[self.cursor.0] = String::from(replace_line);
                self.content.insert(self.cursor.0 + 1, new_line);
            } else {
                self.content.insert(self.cursor.0 + 1, String::new())
            }
            self.cursor.0 += 1;
            self.cursor.1 = 0;
        }
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
