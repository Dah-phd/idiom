use crate::workspace::cursor::Cursor;

use super::TextLine;

pub struct Context<'a> {
    cursor: &'a mut Cursor,
}

impl<'a> Context<'a> {
    fn collect(cursor: &'a mut Cursor, content: &Vec<TextLine>) -> Self {
        Self { cursor }
    }
}
