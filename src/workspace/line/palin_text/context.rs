use crate::workspace::cursor::Cursor;

pub struct Context<'a> {
    cursor: &'a Cursor,
}

impl<'a> Context<'a> {
    fn collect(cursor: &'a Cursor) -> Self {
        Self { cursor }
    }
}
