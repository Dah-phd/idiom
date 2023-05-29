#[derive(Debug)]
pub enum Select {
    None,
    Range((usize, usize), (usize, usize)),
}

type Range = (usize, usize);

impl Default for Select {
    fn default() -> Self {
        Self::None
    }
}

impl Select {
    pub fn is_empty(&self) -> bool {
        matches!(self, Select::None)
    }

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

    pub fn get(&self) -> Option<(&Range, &Range)> {
        match self {
            Self::None => None,
            Self::Range(from, to) => {
                if from.0 > to.0 || from.0 == to.0 && from.1 > to.1 {
                    Some((to, from))
                } else {
                    Some((from, to))
                }
            }
        }
    }
}
