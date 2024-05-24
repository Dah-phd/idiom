use std::io::Write;

use super::{BackendProtocol, Style};

pub struct Backend();

impl BackendProtocol for Backend {
    fn clear_all(&mut self) {}
    fn clear_line(&mut self) {}
    fn clear_to_eol(&mut self) {}

    fn exit() -> std::io::Result<()> {
        Ok(())
    }

    fn get_style(&mut self) -> super::Style {
        Style::default()
    }

    fn go_to(&mut self, _row: u16, _col: u16) {}

    fn hide_cursor(&mut self) {}

    fn init() -> Self {
        Self()
    }

    fn print<D: std::fmt::Display>(&mut self, _text: D) {}

    fn print_at<D: std::fmt::Display>(&mut self, _row: u16, _col: u16, _text: D) {}
    fn print_styled<D: std::fmt::Display>(&mut self, _text: D, _style: Style) {}
    fn print_styled_at<D: std::fmt::Display>(&mut self, _row: u16, _col: u16, _text: D, _style: Style) {}

    fn render_cursor_at(&mut self, _row: u16, _col: u16) {}

    fn reset_style(&mut self) {}
    fn restore_cursor(&mut self) {}
    fn save_cursor(&mut self) {}

    fn screen() -> std::io::Result<crate::render::layout::Rect> {
        Ok(crate::render::layout::Rect::new(0, 0, 120, 60))
    }

    fn set_bg(&mut self, _color: Option<super::Color>) {}
    fn set_fg(&mut self, _color: Option<super::Color>) {}
    fn set_style(&mut self, _style: Style) {}
    fn show_cursor(&mut self) {}

    fn to_set_style(&mut self) {}

    fn update_style(&mut self, _style: Style) {}
}

impl Write for Backend {
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write_all(&mut self, mut _buf: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
}
