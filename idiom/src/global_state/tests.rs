use super::GlobalState;
use crate::embeded_term::EditorTerminal;
use crate::tree::tests::mock_tree;
use crate::workspace::tests::mock_ws;

use std::io::Write;

use crossterm::style::{Color, ContentStyle};

use idiom_ui::{
    backend::{Backend, StyleExt},
    layout::{Borders, Line, Rect},
};

pub struct MockedBackend {
    pub data: Vec<(ContentStyle, String)>,
    pub default_style: ContentStyle,
}

impl Backend for MockedBackend {
    fn init() -> Self {
        Self { data: Vec::new(), default_style: ContentStyle::default() }
    }

    fn exit() -> std::io::Result<()> {
        Ok(())
    }

    fn freeze(&mut self) {
        self.data.push((ContentStyle::default(), String::from("<<freeze>>")));
    }

    fn unfreeze(&mut self) {
        self.data.push((ContentStyle::default(), String::from("<<unfreeze>>")));
    }

    /// force flush buffer if writing small amount of data
    fn flush_buf(&mut self) {}

    fn clear_all(&mut self) {
        self.data.push((ContentStyle::default(), String::from("<<clear all>>")));
    }
    fn clear_line(&mut self) {
        self.data.push((ContentStyle::default(), String::from("<<clear line>>")));
    }
    fn clear_to_eol(&mut self) {
        self.data.push((ContentStyle::default(), String::from("<<clear EOL>>")));
    }

    fn get_style(&mut self) -> ContentStyle {
        self.default_style
    }

    fn go_to(&mut self, row: u16, col: u16) {
        self.data.push((ContentStyle::default(), format!("<<go to row: {row} col: {col}>>")))
    }

    fn hide_cursor(&mut self) {}

    fn print<D: std::fmt::Display>(&mut self, text: D) {
        self.data.push((self.default_style, text.to_string()));
    }

    fn print_at<D: std::fmt::Display>(&mut self, row: u16, col: u16, text: D) {
        self.go_to(row, col);
        self.print(text)
    }
    fn print_styled<D: std::fmt::Display>(&mut self, text: D, style: ContentStyle) {
        self.data.push((style, text.to_string()));
    }

    fn print_styled_at<D: std::fmt::Display>(&mut self, row: u16, col: u16, text: D, style: ContentStyle) {
        self.go_to(row, col);
        self.print_styled(text, style);
    }

    fn render_cursor_at(&mut self, row: u16, col: u16) {
        self.data.push((self.default_style, format!("<<draw cursor row: {row} col: {col}>>")));
    }

    fn reset_style(&mut self) {
        self.default_style = ContentStyle::default();
        self.data.push((self.default_style, String::from("<<reset style>>")));
    }

    fn restore_cursor(&mut self) {
        self.data.push((self.default_style, String::from("<<restored cursor>>")))
    }

    fn save_cursor(&mut self) {
        self.data.push((self.default_style, String::from("<<saved cursor>>")));
    }

    fn screen() -> std::io::Result<idiom_ui::layout::Rect> {
        Ok(idiom_ui::layout::Rect::new(0, 0, 120, 60))
    }

    fn set_bg(&mut self, color: Option<Color>) {
        self.default_style.set_bg(color);
        self.data.push((self.default_style, format!("<<set bg {:?}>>", color)));
    }

    fn set_fg(&mut self, color: Option<Color>) {
        self.default_style.set_fg(color);
        self.data.push((self.default_style, format!("<<set fg {:?}>>", color)));
    }

    fn set_style(&mut self, style: ContentStyle) {
        self.default_style = style;
        self.data.push((self.default_style, "<<set style>>".to_string()))
    }

    fn show_cursor(&mut self) {}

    fn to_set_style(&mut self) {
        self.data.push((self.default_style, String::from("<<set style>>")));
    }

    fn update_style(&mut self, style: ContentStyle) {
        self.default_style.update(style);
        self.data.push((self.default_style, String::from("<<updated style>>")))
    }

    fn pad(&mut self, width: usize) {
        self.data.push((self.default_style, format!("<<padding: {:?}>>", width)))
    }

    fn pad_styled(&mut self, width: usize, style: ContentStyle) {
        self.data.push((self.default_style, format!("<<padding: {:?}, styled: {:?}>>", width, style)))
    }
}

impl Write for MockedBackend {
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

impl MockedBackend {
    pub fn unwrap(self) -> Vec<(ContentStyle, String)> {
        self.data
    }

    pub fn drain(&mut self) -> Vec<(ContentStyle, String)> {
        std::mem::take(&mut self.data)
    }
}

pub fn mocked_global() -> GlobalState {
    let backend = MockedBackend { data: vec![], default_style: ContentStyle::default() };
    GlobalState::new(Rect::new(0, 0, 120, 60), backend)
}

#[test]
fn full_rebuild_draw() {
    let mut gs = mocked_global();
    let mut ws = mock_ws(
        ["test line uno - in here", "second line", "last line for the test"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    );
    let mut tree = mock_tree();
    let mut term = EditorTerminal::new(Some(String::new()));
    gs.full_resize(80, 80);
    let editor_rect = gs.calc_editor_rect();
    gs.draw(&mut ws, &mut tree, &mut term);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(editor_rect, gs.editor_area);
    assert_eq!(gs.editor_area, Rect { row: 1, col: 14, width: 66, height: 78, borders: Borders::empty() });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 14, width: 66, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 1, col: 1, width: 12, height: 78, borders: Borders::LEFT | Borders::RIGHT });
    assert_eq!(gs.footer_line, Line { row: 80, col: 0, width: 80 });
}

#[test]
fn full_rebuild_draw_insert() {
    let mut gs = mocked_global();
    gs.toggle_tree();
    gs.insert_mode();
    let mut ws = mock_ws(
        ["test line uno - in here", "second line", "last line for the test"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    );
    let mut tree = mock_tree();
    let mut term = EditorTerminal::new(Some(String::new()));
    gs.full_resize(80, 80);
    let editor_rect = gs.calc_editor_rect();
    gs.draw(&mut ws, &mut tree, &mut term);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(editor_rect, gs.editor_area);
    assert_eq!(gs.editor_area, Rect { row: 1, col: 0, width: 80, height: 78, borders: Borders::empty() });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 0, width: 80, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 0, col: 0, width: 0, height: 79, borders: Borders::empty() });
    assert_eq!(gs.footer_line, Line { row: 80, col: 0, width: 80 });
}
