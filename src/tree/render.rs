use super::{FileClipboard, Tree, TreePath};
use crate::ext_tui::{CrossTerm, StyleExt};
use crate::global_state::GlobalState;
use crossterm::style::ContentStyle;
use idiom_tui::{layout::Line, Backend};

const UP: char = '⭡';
const DOWN: char = '⭣';
const ARROW_WIDTH: usize = 1;
const MIN_OFFSET: usize = 10;
const MIN_ARROW_LINE_WIDTH: usize = ARROW_WIDTH + MIN_OFFSET;

pub fn render_tree(tree: &mut Tree, gs: &mut GlobalState) {
    let mut iter = tree.inner.iter();
    iter.next();

    let mut tree_area = gs.tree_area.clone();
    let mut last_line = tree_area.pop_line();
    let mut lines = tree_area.into_iter();

    let base_style = gs.theme.accent_style;
    let select_base_style = tree.state.highlight.with_bg(gs.theme.accent_background);

    let mut tree_iter = iter.enumerate().skip(tree.state.at_line);

    if tree.state.at_line != 0 {
        if let Some(mut first) = lines.next() {
            if let Some((idx, tree_path)) = tree_iter.next() {
                let style = match idx == tree.state.selected {
                    true => select_base_style,
                    false => base_style,
                };
                if MIN_ARROW_LINE_WIDTH < first.width {
                    first.width -= ARROW_WIDTH;
                    gs.backend.print_styled_at(first.row, first.col + first.width as u16, UP, style);
                }
                print_styled_path(tree_path, first, style, tree.display_offset, &tree.tree_clipboard, &mut gs.backend);
            }
        }
    }

    for (idx, tree_path) in &mut tree_iter {
        let style = match idx == tree.state.selected {
            true => select_base_style,
            false => base_style,
        };

        let Some(line) = lines.next() else {
            if tree_iter.next().is_some() && MIN_ARROW_LINE_WIDTH < last_line.width {
                last_line.width -= ARROW_WIDTH;
                gs.backend.print_styled_at(last_line.row, last_line.col + last_line.width as u16, DOWN, style);
            }
            print_styled_path(tree_path, last_line, style, tree.display_offset, &tree.tree_clipboard, &mut gs.backend);
            return;
        };

        print_styled_path(tree_path, line, style, tree.display_offset, &tree.tree_clipboard, &mut gs.backend);
    }
    for line in lines {
        line.fill_styled(' ', base_style, &mut gs.backend);
    }
    last_line.fill_styled(' ', base_style, &mut gs.backend);
}

fn print_styled_path(
    tree_path: &TreePath,
    mut line: Line,
    style: ContentStyle,
    offset: usize,
    clipboard: &FileClipboard,
    backend: &mut CrossTerm,
) {
    if let Some(mark) = clipboard.get_mark(tree_path.path()) {
        if mark.len() + MIN_OFFSET < line.width {
            line.width -= mark.len();
            backend.print_styled_at(line.row, line.col + line.width as u16, mark, style);
        }
    }
    tree_path.render(offset, line, style, backend);
}

#[cfg(test)]
mod test {
    use super::{ARROW_WIDTH, DOWN, MIN_ARROW_LINE_WIDTH, MIN_OFFSET, UP};
    use idiom_tui::UTF8Safe;

    #[test]
    fn ensure_const_fit() {
        let up_w = UP.to_string().width();
        let down_w = DOWN.to_string().width();
        assert_eq!(up_w, ARROW_WIDTH);
        assert_eq!(down_w, ARROW_WIDTH);
        assert_eq!(MIN_ARROW_LINE_WIDTH, ARROW_WIDTH + MIN_OFFSET);
    }
}
