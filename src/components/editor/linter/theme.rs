use tui::style::Color;

pub struct Theme {
    pub kword: Color,
    pub class: Color,
    pub function: Color,
    pub blank: Color,
    pub default: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            kword: Color::Rgb(69, 84, 113),
            class: Color::Rgb(101, 155, 111),
            default: Color::Rgb(130, 165, 187),
            function: Color::Rgb(175, 153, 90),
            blank: Color::White,
        }
    }
}
