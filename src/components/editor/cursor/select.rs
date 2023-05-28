#[derive(Debug)]
pub enum Select {
    None,
    Range((usize, usize), (usize, usize)),
    MultiRange(Vec<((usize, usize), (usize, usize))>),
}

impl Default for Select {
    fn default() -> Self {
        Self::None
    }
}

impl Select {
    pub fn drop(&mut self) {
        (*self) = Self::None;
    }

    pub fn init(&mut self, line: usize, char: usize) {
        if matches!(self, Select::None) {
            (*self) = Self::Range((line, char), (line, char))
        }
    }

    pub fn push(&mut self, line: usize, char: usize) {
        if let Self::Range(_, to) = self {
            (*to) = (line, char)
        }
    }
}

#[derive(Debug)]
pub enum Clip {
    Line(String),
    Text(String),
    Section(Vec<String>),
}
