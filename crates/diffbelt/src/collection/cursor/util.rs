pub trait BaseCursor {
    fn prev_cursor_id(&self) -> Option<&str>;
    fn next_cursor_id(&self) -> Option<&str>;
    fn set_next_cursor_id(&mut self, id: String) -> ();
}
