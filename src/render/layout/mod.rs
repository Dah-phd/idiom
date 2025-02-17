#[allow(dead_code)]
mod borders;
mod line;
mod rect;
mod rect_iter;

pub use rect::Rect;
pub use rect_iter::{DoublePaddedRectIter, IterLines, RectIter};
#[allow(unused_imports)]
pub use {
    borders::{
        BorderSet, Borders, BORDERS, DOUBLE_BORDERS, FULL_BORDERS, HAVED_THIN_BORDERS, HAVED_WIDE_BORDERS,
        HAVLED_BALANCED_BORDERS, THICK_BORDERS,
    },
    line::{Line, LineBuilder, LineBuilderRev},
};
