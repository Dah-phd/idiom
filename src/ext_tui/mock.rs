use super::StyleExt;
use crossterm::style::{Color, ContentStyle};
use idiom_tui::{backend::Backend, layout::Rect};
use std::io::Write;

#[derive(Debug, Default)]
pub struct MockedBackend {
    pub data: Vec<(ContentStyle, String)>,
    pub default_style: ContentStyle,
}

impl PartialEq for MockedBackend {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl MockedBackend {
    pub fn detached_hide_cursor() {}
    pub fn detached_show_cursor() {}
}

impl Backend for MockedBackend {
    type Style = ContentStyle;
    type Color = Color;

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

    fn screen() -> std::io::Result<Rect> {
        Ok(Rect::new(0, 0, 120, 60))
    }

    fn set_bg(&mut self, color: Option<Color>) {
        self.default_style.set_bg(color);
        self.data.push((self.default_style, format!("<<set bg {color:?}>>")));
    }

    fn set_fg(&mut self, color: Option<Color>) {
        self.default_style.set_fg(color);
        self.data.push((self.default_style, format!("<<set fg {color:?}>>")));
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
        self.data.push((self.default_style, format!("<<padding: {width:?}>>")))
    }

    fn pad_styled(&mut self, width: usize, style: ContentStyle) {
        self.data.push((self.default_style, format!("<<padding: {width:?}, styled: {style:?}>>")))
    }

    fn merge_style(mut left: Self::Style, right: Self::Style) -> Self::Style {
        left.update(right);
        left
    }

    fn reversed_style() -> Self::Style {
        ContentStyle::reversed()
    }

    fn bold_style() -> Self::Style {
        ContentStyle::bold()
    }

    fn ital_style() -> Self::Style {
        ContentStyle::ital()
    }

    fn fg_style(color: Self::Color) -> Self::Style {
        ContentStyle::fg(color)
    }

    fn bg_style(color: Self::Color) -> Self::Style {
        ContentStyle::bg(color)
    }

    fn slow_blink_style() -> Self::Style {
        ContentStyle::slowblink()
    }

    fn underline_style(color: Option<Self::Color>) -> Self::Style {
        ContentStyle::underlined(color)
    }

    fn undercurle_style(color: Option<Self::Color>) -> Self::Style {
        ContentStyle::undercurled(color)
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
