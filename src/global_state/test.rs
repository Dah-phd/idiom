use std::io::Write;

use crate::{configs::UITheme, render::layout::Rect};

use super::{controls, draw::Components, Clipboard, GlobalState, Mode};

pub fn mock_gs() -> GlobalState {
    let screen_rect = Rect::new(0, 0, 120, 60);
    let mut new = GlobalState {
        mode: Mode::default(),
        tree_size: 15,
        key_mapper: controls::map_tree,
        mouse_mapper: controls::mouse_handler,
        theme: UITheme::new().unwrap_or_default(),
        writer: DummyOut,
        popup: None,
        footer: Vec::default(),
        workspace: Vec::default(),
        tree: Vec::default(),
        clipboard: Clipboard::default(),
        exit: false,
        screen_rect,
        tree_area: Rect::default(),
        tab_area: Rect::default(),
        editor_area: Rect::default(),
        footer_area: Rect::default(),
        components: Components::default(),
    };
    new.recalc_draw_size();
    new.select_mode();
    new
}

struct DummyOut();

impl Write for DummyOut {
    fn by_ref(&mut self) -> &mut Self
        where
            Self: Sized, {
        self
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn write_all(&mut self, mut buf: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        Ok(())
    }

    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        Ok((bufs.len()))
    }
}