use tui::style::Color;

#[derive(Debug)]
pub struct Theme {
    pub kword: Color,
    pub class: Color,
    pub function: Color,
    pub blank: Color,
    pub default: Color,
    pub selected: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            kword: Color::Rgb(79, 106, 214),
            class: Color::Rgb(112, 199, 176),
            default: Color::Rgb(108, 149, 214),
            function: Color::Rgb(218, 223, 170),
            blank: Color::White,
            selected: Color::Rgb(72, 72, 72),
        }
    }
}
