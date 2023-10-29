use ratatui::style::Color;

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
