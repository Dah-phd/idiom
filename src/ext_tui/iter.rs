use idiom_tui::layout::{Line, RectIter};

pub struct TakeLiens<'a> {
    take: usize,
    inner: &'a mut RectIter,
}

impl<'a> TakeLiens<'a> {
    pub fn new(iter: &'a mut RectIter, take: usize) -> Self {
        Self { inner: iter, take }
    }
}

impl<'a> Iterator for TakeLiens<'a> {
    type Item = Line;
    fn next(&mut self) -> Option<Self::Item> {
        if self.take == 0 {
            return None;
        }
        self.take -= 1;
        self.inner.next()
    }
}

#[cfg(test)]
mod tests {
    use super::TakeLiens;
    use idiom_tui::layout::{Line, Rect};

    #[test]
    fn test_take_and_unwrap() {
        let rect = Rect::new(0, 0, 20, 20);
        let mut iter = rect.into_iter();
        assert_eq!(iter.next(), Some(Line { row: 0, col: 0, width: 20 }));
        let mut take = TakeLiens::new(&mut iter, 2);
        assert_eq!(take.next(), Some(Line { row: 1, col: 0, width: 20 }));
        assert_eq!(take.next(), Some(Line { row: 2, col: 0, width: 20 }));
        assert_eq!(take.next(), None);
        assert_eq!(iter.next(), Some(Line { row: 3, col: 0, width: 20 }));
    }
}
