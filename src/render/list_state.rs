use ratatui::widgets::ListState;

#[derive(Default)]
pub struct WrappedState {
    inner: ListState,
}

impl WrappedState {
    pub fn set(&mut self, idx: usize) {
        self.inner.select(Some(idx));
    }

    pub fn drop(&mut self) {
        self.inner.select(None);
    }

    pub fn selected(&self) -> Option<usize> {
        self.inner.selected()
    }

    pub fn get(&mut self) -> &mut ListState {
        &mut self.inner
    }

    pub fn next<T>(&mut self, options: &[T]) {
        match self.inner.selected() {
            Some(idx) => {
                let idx = idx + 1;
                self.inner.select(Some(if idx < options.len() { idx } else { 0 }));
            }
            None if !options.is_empty() => self.inner.select(Some(0)),
            _ => (),
        }
    }

    pub fn prev<T>(&mut self, options: &[T]) {
        match self.inner.selected() {
            Some(idx) => self.inner.select(Some(if idx == 0 { options.len() - 1 } else { idx - 1 })),
            None => self.inner.select(Some(options.len() - 1)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::WrappedState;

    #[test]
    fn test_setting() {
        let mut ls = WrappedState::default();
        ls.set(3);
        assert_eq!(ls.inner.selected(), Some(3));
        ls.set(2);
        assert_eq!(ls.inner.selected(), Some(2));
        ls.drop();
        assert_eq!(ls.inner.selected(), None);
    }

    #[test]
    fn test_movement() {
        let mut ls = WrappedState::default();
        let options = [1, 2, 3];
        ls.next(&options);
        assert_eq!(ls.selected(), Some(0));
        ls.next(&options);
        assert_eq!(ls.selected(), Some(1));
        ls.next(&options);
        ls.next(&options);
        assert_eq!(ls.selected(), Some(0));
        ls.prev(&options);
        assert_eq!(ls.selected(), Some(2));
    }
}
