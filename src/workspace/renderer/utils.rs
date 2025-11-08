use crate::ext_tui::{CrossTerm, StyleExt};
use crate::global_state::GlobalState;
use crate::syntax::tokens::WrapData;
use crate::workspace::{
    cursor::CharRangeUnbound,
    line::{EditorLine, LineContext},
};
use crossterm::style::{Color, ContentStyle};
use idiom_tui::{
    layout::{IterLines, RectIter},
    Backend,
};

pub struct SelectManagerSimple {
    unbound: bool,
    from: usize,
    to: usize,
    select_color: Color,
    select_style_set: fn(usize, &mut CrossTerm, &mut Self),
    pad: fn(&mut GlobalState),
}

impl SelectManagerSimple {
    pub fn new(select: CharRangeUnbound, select_color: Color) -> Option<Self> {
        match (select.from, select.to) {
            (.., Some(0)) => None,
            (Some(from), Some(to)) => {
                Some(Self { unbound: false, from, to, select_color, select_style_set: select_style_no_reset, pad })
            }
            (Some(from), None) => Some(Self {
                unbound: false,
                from,
                to: usize::MAX,
                select_color,
                select_style_set: select_style_no_end_no_reset,
                pad: pad_select,
            }),
            (None, Some(to)) => {
                Some(Self { unbound: false, from: 0, to, select_color, select_style_set: select_style_no_reset, pad })
            }
            (None, None) => Some(Self {
                unbound: true,
                from: 0,
                to: usize::MAX,
                select_color,
                select_style_set: select_style_no_end_no_reset,
                pad: pad_select,
            }),
        }
    }

    #[inline]
    pub fn go_to_index(&mut self, idx: usize, backend: &mut CrossTerm) {
        // whole line is selected
        if self.unbound {
            backend.set_bg(Some(self.select_color));
            self.select_style_set = select_style_no_op_no_reset;
            return;
        }
        // idx is before select start
        if self.from > idx {
            return;
        }
        // end is unbound - set and forget
        if self.to == usize::MAX {
            backend.set_bg(Some(self.select_color));
            self.select_style_set = select_style_no_op_no_reset;
            return;
        }
        // idx is after end -> no on
        if idx >= self.to {
            self.select_style_set = select_style_no_op_no_reset;
            return;
        }
        backend.set_bg(Some(self.select_color));
        self.select_style_set = select_style_post_start_no_reset;
    }

    #[inline(always)]
    pub fn set_style(&mut self, idx: usize, backend: &mut CrossTerm) {
        (self.select_style_set)(idx, backend, self)
    }

    #[inline(always)]
    pub fn pad(&self, gs: &mut GlobalState) {
        (self.pad)(gs)
    }
}

pub struct SelectManager {
    unbound: bool,
    from: usize,
    to: usize,
    select_color: Color,
    select_style_set: fn(usize, &mut CrossTerm, &mut ContentStyle, &mut Self),
    pad: fn(&mut GlobalState),
}

impl SelectManager {
    pub fn new(select: CharRangeUnbound, select_color: Color) -> Option<Self> {
        match (select.from, select.to) {
            (.., Some(0)) => None,
            (Some(from), Some(to)) => {
                Some(Self { unbound: false, from, to, select_color, select_style_set: select_style, pad })
            }
            (None, Some(to)) => {
                Some(Self { unbound: false, from: 0, to, select_color, select_style_set: select_style, pad })
            }
            (Some(from), None) => Some(Self {
                unbound: false,
                from,
                to: usize::MAX,
                select_color,
                select_style_set: select_style_no_end,
                pad: pad_select,
            }),
            (None, None) => Some(Self {
                unbound: true,
                from: 0,
                to: usize::MAX,
                select_color,
                select_style_set: select_style_no_end,
                pad: pad_select,
            }),
        }
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.from
    }

    #[inline]
    pub fn go_to_index(&mut self, idx: usize, reset_style: &mut ContentStyle, backend: &mut CrossTerm) {
        // whole line is selected
        if self.unbound {
            reset_style.set_bg(Some(self.select_color));
            backend.set_bg(Some(self.select_color));
            self.select_style_set = select_style_no_op;
            return;
        }
        // idx is before select start
        if self.from > idx {
            return;
        }
        // end is unbound - set and forget
        if self.to == usize::MAX {
            reset_style.set_bg(Some(self.select_color));
            backend.set_bg(Some(self.select_color));
            self.select_style_set = select_style_no_op;
            return;
        }
        // idx is after end -> no on
        if idx >= self.to {
            self.select_style_set = select_style_no_op;
            return;
        }
        reset_style.set_bg(Some(self.select_color));
        backend.set_bg(Some(self.select_color));
        self.select_style_set = select_style_post_start;
    }

    #[inline(always)]
    pub fn set_style(&mut self, idx: usize, reset_style: &mut ContentStyle, backend: &mut CrossTerm) {
        (self.select_style_set)(idx, backend, reset_style, self)
    }

    #[inline(always)]
    pub fn pad(&self, gs: &mut GlobalState) {
        (self.pad)(gs)
    }
}

fn select_style(idx: usize, backend: &mut CrossTerm, reset_style: &mut ContentStyle, select: &mut SelectManager) {
    if select.from == idx {
        backend.set_bg(Some(select.select_color));
        reset_style.set_bg(Some(select.select_color));
        select.select_style_set = select_style_post_start;
    }
}

fn select_style_post_start(
    idx: usize,
    backend: &mut CrossTerm,
    reset_style: &mut ContentStyle,
    select: &mut SelectManager,
) {
    if select.to == idx {
        backend.set_bg(None);
        reset_style.set_bg(None);
        // disalbe select setter
        select.select_style_set = select_style_no_op;
    }
}

fn select_style_no_end(
    idx: usize,
    backend: &mut CrossTerm,
    reset_style: &mut ContentStyle,
    select: &mut SelectManager,
) {
    if select.from == idx {
        backend.set_bg(Some(select.select_color));
        reset_style.set_bg(Some(select.select_color));
        select.select_style_set = select_style_no_op;
    }
}

fn select_style_no_op(_: usize, _: &mut CrossTerm, _: &mut ContentStyle, _: &mut SelectManager) {}

fn select_style_no_reset(idx: usize, backend: &mut CrossTerm, select: &mut SelectManagerSimple) {
    if select.from == idx {
        backend.set_bg(Some(select.select_color));
        select.select_style_set = select_style_post_start_no_reset;
    }
}

fn select_style_post_start_no_reset(idx: usize, backend: &mut CrossTerm, select: &mut SelectManagerSimple) {
    if select.to == idx {
        backend.set_bg(None);
        // disalbe select setter
        select.select_style_set = select_style_no_op_no_reset;
    }
}

fn select_style_no_end_no_reset(idx: usize, backend: &mut CrossTerm, select: &mut SelectManagerSimple) {
    if select.from == idx {
        backend.set_bg(Some(select.select_color));
        select.select_style_set = select_style_no_op_no_reset;
    }
}

fn select_style_no_op_no_reset(_: usize, _: &mut CrossTerm, _: &mut SelectManagerSimple) {}

pub fn pad(gs: &mut GlobalState) {
    gs.backend.print(" ")
}

pub fn pad_select(gs: &mut GlobalState) {
    let select_style = gs.get_accented_select();
    gs.backend().print_styled("~", select_style);
}

#[inline]
pub fn try_cache_wrap_data_from_lines(
    text: &mut EditorLine,
    len_pre_render: usize,
    lines: &RectIter,
    ctx: &LineContext,
) {
    // line could have been partially rendered
    let len_post_render = lines.len();
    if len_post_render == 0 {
        return;
    }
    let text_width = lines.width() - ctx.line_prefix_len();
    WrapData::new(len_pre_render - len_post_render, text_width).store(text);
}

#[cfg(test)]
mod test {
    use super::SelectManager;
    use crate::ext_tui::CrossTerm;
    use crate::global_state::GlobalState;
    use crate::workspace::cursor::CharRangeUnbound;
    use crate::workspace::editor::tests::mock_editor_text_render;
    use crossterm::style::{Color, ContentStyle};
    use idiom_tui::{layout::Rect, Backend};

    #[test]
    fn test_go_to_index() {
        let select_color = Color::Black;
        let range = CharRangeUnbound { from: Some(10), to: None };
        let mut reset_style = ContentStyle::default();
        let mut backend = CrossTerm::init();

        let mut sel = SelectManager::new(range, select_color).unwrap();
        sel.go_to_index(5, &mut reset_style, &mut backend);
        assert_eq!(reset_style.background_color, None);
        sel.go_to_index(10, &mut reset_style, &mut backend);
        assert_eq!(reset_style.background_color, Some(select_color));
        sel.go_to_index(100, &mut reset_style, &mut backend);
        assert_eq!(reset_style.background_color, Some(select_color));
    }

    #[test]
    fn test_build() {
        let select_color = Color::Black;
        let range = CharRangeUnbound { from: Some(4), to: Some(15) };
        let mut reset_style = ContentStyle::default();
        let mut backend = CrossTerm::init();

        let mut sel = SelectManager::new(range, select_color).unwrap();
        for idx in 0..20 {
            sel.set_style(idx, &mut reset_style, &mut backend);
            if (4..15).contains(&idx) {
                assert_eq!(reset_style.background_color, Some(select_color));
            } else {
                assert_eq!(reset_style.background_color, None);
            }
        }
    }

    #[test]
    fn test_try_wrap_data_from_lines() {
        let mut gs = GlobalState::new(Rect::new(0, 0, 25, 5), CrossTerm::init());
        gs.force_area_calc();
        let mut editor = mock_editor_text_render(vec![
            "let mut gs = GlobalState::new(Rect::new(0, 0, 30, 60), CrossTerm::init());".into(),
            "n/a".into(),
            "n/a".into(),
        ]);
        editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
        todo!()
    }
}
