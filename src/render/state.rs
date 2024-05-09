use crate::render::layout::Rect;
use std::io::Write;

use super::{
    backend::{Backend, BackendProtocol, Style},
    layout::LineBuilder,
};

pub struct State {
    pub at_line: usize,
    pub selected: usize,
    pub highlight: Style,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl State {
    pub fn new() -> Self {
        let highlight = Style::reversed();
        Self { at_line: 0, selected: 0, highlight }
    }

    pub fn with_highlight(highlight: Style) -> Self {
        Self { at_line: 0, selected: 0, highlight }
    }

    pub fn select(&mut self, idx: usize, option_len: usize) {
        if option_len > idx {
            self.at_line = idx;
            self.selected = idx;
        }
    }

    pub fn next(&mut self, option_len: usize) {
        self.selected += 1;
        if self.selected >= option_len {
            self.selected = 0;
        };
    }

    pub fn prev(&mut self, option_len: usize) {
        if self.selected > 0 {
            self.selected -= 1;
        } else if option_len > 0 {
            self.selected = option_len - 1;
        };
    }

    #[inline]
    pub fn update_at_line(&mut self, limit: usize) {
        if self.at_line > self.selected {
            self.at_line = self.selected;
        } else if self.selected - self.at_line >= limit {
            self.at_line = self.selected - limit + 1;
        };
    }

    #[inline]
    pub fn render_list_complex<T>(
        &mut self,
        options: &[T],
        callbacks: &[fn(&T, builder: LineBuilder) -> std::io::Result<()>],
        rect: &Rect,
        backend: &mut Backend,
    ) -> std::io::Result<()> {
        let limit = rect.height as usize / callbacks.len();
        self.update_at_line(limit);
        let mut lines = rect.into_iter();
        for (idx, option) in options.iter().enumerate().skip(self.at_line) {
            if idx == self.selected {
                backend.set_style(self.highlight)?;
                for callback in callbacks {
                    match lines.next() {
                        Some(line) => {
                            (callback)(option, line.unsafe_builder(backend)?)?;
                        }
                        None => break,
                    };
                }
                backend.reset_style()?;
                continue;
            };
            for callback in callbacks {
                match lines.next() {
                    Some(line) => {
                        (callback)(option, line.unsafe_builder(backend)?)?;
                    }
                    None => break,
                };
            }
        }
        backend.reset_style()?;
        for line in lines {
            line.render_empty(backend)?;
        }
        Ok(())
    }

    #[inline]
    pub fn render_list_styled<'a>(
        &mut self,
        options: impl Iterator<Item = (&'a str, Style)>,
        rect: &Rect,
        writer: &mut Backend,
    ) -> std::io::Result<()> {
        self.update_at_line(rect.height as usize);
        let mut lines = rect.into_iter();
        for (idx, (text, mut style)) in options.enumerate().skip(self.at_line) {
            let line = match lines.next() {
                Some(line) => line,
                None => break,
            };
            if idx == self.selected {
                style.update(self.highlight);
            }
            line.render_styled(text, style, writer)?;
        }
        for line in lines {
            line.render_empty(writer)?;
        }
        writer.flush()
    }

    pub fn render_list<'a>(
        &mut self,
        options: impl Iterator<Item = &'a str>,
        rect: &Rect,
        writer: &mut Backend,
    ) -> std::io::Result<()> {
        self.update_at_line(rect.height as usize);
        let mut lines = rect.into_iter();
        for (idx, text) in options.enumerate().skip(self.at_line) {
            let line = match lines.next() {
                Some(line) => line,
                None => break,
            };
            if idx == self.selected {
                line.render_styled(text, self.highlight, writer)?;
            } else {
                line.render(text, writer)?;
            };
        }
        for line in lines {
            line.render_empty(writer)?;
        }
        writer.flush()
    }
}
