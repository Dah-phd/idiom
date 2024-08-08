use crate::workspace::cursor::Cursor;

pub struct Context<'a> {
    cursor: &'a mut Cursor,
}

impl<'a> Context<'a> {
    fn collect(cursor: &'a mut Cursor) -> Self {
        Self { cursor }
    }

    fn find_position() {}
}
