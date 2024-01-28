use ratatui::prelude::Rect;
type Line = usize;
type Column = usize;

pub fn is_in(rect: Rect, row: u16, column: u16) -> bool {
    // x horizontal
    // y vertical
    // MouseEvent { kind: Up(Left), column: 24, row: 2, modifiers: KeyModifiers(0x0) }
    // NOT Rect { x: 29, y: 1, width: 166, height: 41 }
    // MouseEvent { kind: Up(Left), column: 46, row: 3, modifiers: KeyModifiers(0x0) }
    // NOT Rect { x: 29, y: 1, width: 166, height: 41 }
    rect.x <= column && column <= rect.width && rect.y <= row && row <= rect.height
}

pub fn solve_position(rect: Rect, row: u16, column: u16) -> (Line, Column) {
    ((row - rect.y) as usize, (column - rect.x) as usize)
}

pub fn contained_position(rect: Rect, row: u16, column: u16) -> Option<(Line, Column)> {
    if rect.x <= column && column <= rect.width && rect.y <= row && row <= rect.height {
        return Some(((row - rect.y) as usize, (column - rect.x) as usize));
    }
    None
}
