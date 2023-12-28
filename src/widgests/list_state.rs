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
