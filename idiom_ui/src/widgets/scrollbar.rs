use std::ops::Range;

#[derive(Debug, PartialEq)]
pub struct ScrollBar {
    x: u16,
    y: u16,
    screen: usize,
    at_line: usize,
    full_len: usize,
    range: Range<usize>,
}

impl ScrollBar {
    pub fn new(x: u16, y: u16, screen: usize, at_line: usize, full_len: usize) -> Option<Self> {
        if full_len <= screen {
            return None;
        }
        let scale = (screen * 100) / full_len;
        let screen_part = (screen * scale) / 100;
        let start = (at_line * scale) / 100;
        let range = start..(start + screen_part);
        Some(Self { x, y, screen, at_line, full_len, range })
    }

    fn check(&self, idx: usize) -> bool {
        self.range.start <= idx && idx < self.range.end
    }
}

pub struct ScrollIter {
    idx: usize,
    screen: usize,
    at_line: usize,
    full_len: usize,
    range: Range<usize>,
}

impl ScrollIter {
    pub fn new(screen: usize, at_line: usize, full_len: usize) -> Option<Self> {
        if full_len <= screen {
            return None;
        }
        let scale = (screen * 100) / full_len;
        let screen_part = (screen * scale) / 100;
        let start = (at_line * scale) / 100;
        let range = start..(start + screen_part);
        Some(Self { idx: 0, screen, at_line, full_len, range })
    }

    fn check(&self, idx: usize) -> bool {
        self.range.start <= idx && idx < self.range.end
    }

    pub fn skip(&mut self) {
        self.idx += 1;
    }

    pub fn check_next(&mut self) -> bool {
        let result = self.range.start <= self.idx && self.idx < self.range.end;
        self.idx += 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::ScrollBar;

    #[test]
    fn test_scroll() {
        let sb = ScrollBar::new(0, 0, 40, 20, 134).unwrap();
        assert_eq!(sb, ScrollBar { x: 0, y: 0, screen: 40, at_line: 20, full_len: 134, range: 5..16 })
    }
}
