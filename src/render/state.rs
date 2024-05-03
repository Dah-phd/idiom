use crate::render::layout::Rect;
use crossterm::style::{Attribute, ContentStyle};
use std::io::Write;

use super::backend::Backend;

#[derive(Default)]
pub struct State {
    pub at_line: usize,
    pub selected: usize,
    pub highlight: ContentStyle,
}

#[allow(dead_code)]
impl State {
    pub fn new() -> Self {
        let highlight = ContentStyle {
            foreground_color: None,
            background_color: None,
            underline_color: None,
            attributes: Attribute::Reverse.into(),
        };
        Self { at_line: 0, selected: 0, highlight }
    }

    pub fn with_highlight(highlight: ContentStyle) -> Self {
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
    pub fn update_at_line(&mut self, limit: usize, option_len: usize) {
        if option_len <= self.selected {
            self.selected = 0;
            self.at_line = 0;
        } else if self.at_line > self.selected {
            self.at_line = self.selected;
        } else if self.selected - self.at_line >= limit {
            self.at_line = self.selected - limit + 1;
        };
    }

    #[inline]
    pub fn render_styled<'a, D, F>(
        &mut self,
        options: &'a [D],
        rect: &Rect,
        to_str: F,
        style: ContentStyle,
        writer: &mut Backend,
    ) -> std::io::Result<()>
    where
        F: Fn(&'a D) -> &'a str,
    {
        self.update_at_line(rect.height as usize, options.len());
        writer.set_style(style)?;
        for ((idx, text), area) in
            options.iter().map(|d| (to_str)(d)).enumerate().skip(self.at_line).zip(rect.into_iter())
        {
            if idx == self.selected {
                area.render_styled(text, self.highlight, writer)?;
            } else {
                area.render(text, writer)?;
            };
        }
        writer.reset_style()?;
        writer.flush()
    }

    pub fn render<'a, D, F>(
        &mut self,
        options: &'a [D],
        rect: &Rect,
        to_str: F,
        writer: &mut Backend,
    ) -> std::io::Result<()>
    where
        F: Fn(&'a D) -> &'a str,
    {
        self.update_at_line(rect.height as usize, options.len());
        for ((idx, text), area) in
            options.iter().map(|d| (to_str)(d)).enumerate().skip(self.at_line).zip(rect.into_iter())
        {
            if idx == self.selected {
                area.render_styled(text, self.highlight, writer)?;
            } else {
                area.render(text, writer)?;
            };
        }
        writer.flush()
    }

    pub fn render_strings(&mut self, options: &[String], rect: &Rect, writer: &mut Backend) -> std::io::Result<()> {
        self.update_at_line(rect.height as usize, options.len());
        for ((idx, text), area) in options.into_iter().enumerate().skip(self.at_line).zip(rect.into_iter()) {
            if idx == self.selected {
                area.render_styled(text, self.highlight, writer)?;
            } else {
                area.render(text, writer)?;
            };
        }
        writer.flush()
    }

    pub fn render_strs(&mut self, options: &[&str], rect: &Rect, writer: &mut Backend) -> std::io::Result<()> {
        self.update_at_line(rect.height as usize, options.len());
        for ((idx, text), area) in options.into_iter().enumerate().skip(self.at_line).zip(rect.into_iter()) {
            if idx == self.selected {
                area.render_styled(text, self.highlight, writer)?;
            } else {
                area.render(text, writer)?;
            };
        }
        writer.flush()
    }
}
