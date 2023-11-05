use ratatui::{
    style::{Color, Style},
    text::Span,
};

pub const COLORS: [Color; 3] = [Color::LightMagenta, Color::LightYellow, Color::Blue];

#[derive(Debug, Default)]
pub struct BracketColors {
    round: Vec<Color>,
    curly: Vec<Color>,
    square: Vec<Color>,
}

impl BracketColors {
    pub fn reset(&mut self) {
        self.round.clear();
        self.curly.clear();
        self.square.clear();
    }

    pub fn map_style(&mut self, ch: char, style: &mut Style) {
        match ch {
            '(' => {
                style.fg.replace(self.open());
            }
            ')' => {
                style.fg.replace(self.close());
            }
            '[' => {
                style.fg.replace(self.square_open());
            }
            ']' => {
                style.fg.replace(self.square_close());
            }
            '{' => {
                style.fg.replace(self.curly_open());
            }
            '}' => {
                style.fg.replace(self.curly_close());
            }
            _ => (),
        };
    }

    pub fn map(&mut self, ch: char, style: Style) -> Option<Span<'static>> {
        match ch {
            '(' => Some(Span::styled(ch.to_string(), style.fg(self.open()))),
            ')' => Some(Span::styled(ch.to_string(), style.fg(self.close()))),
            '[' => Some(Span::styled(ch.to_string(), style.fg(self.square_open()))),
            ']' => Some(Span::styled(ch.to_string(), style.fg(self.square_close()))),
            '{' => Some(Span::styled(ch.to_string(), style.fg(self.curly_open()))),
            '}' => Some(Span::styled(ch.to_string(), style.fg(self.curly_close()))),
            _ => None,
        }
    }

    pub fn open(&mut self) -> Color {
        Self::open_bracket(&mut self.round)
    }

    pub fn close(&mut self) -> Color {
        self.round.pop().unwrap_or(COLORS[COLORS.len() - 1])
    }

    pub fn curly_open(&mut self) -> Color {
        Self::open_bracket(&mut self.curly)
    }

    pub fn curly_close(&mut self) -> Color {
        self.curly.pop().unwrap_or(COLORS[COLORS.len() - 1])
    }

    pub fn square_open(&mut self) -> Color {
        Self::open_bracket(&mut self.square)
    }

    pub fn square_close(&mut self) -> Color {
        self.square.pop().unwrap_or(COLORS[COLORS.len() - 1])
    }

    fn open_bracket(brackets: &mut Vec<Color>) -> Color {
        let color = COLORS[brackets.len() % COLORS.len()];
        brackets.push(color);
        color
    }
}
