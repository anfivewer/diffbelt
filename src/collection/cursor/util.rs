use crate::util::base62::rand_b62;
use std::collections::HashMap;
use std::sync::Arc;

pub trait BaseCursor {
    fn prev_cursor_id(&self) -> Option<&str>;
    fn next_cursor_id(&self) -> Option<&str>;
    fn set_next_cursor_id(&mut self, id: String) -> ();
}

pub fn save_next_cursor<C: BaseCursor>(
    cursors: &std::sync::RwLock<HashMap<String, Arc<std::sync::RwLock<C>>>>,
    current_cursor: Arc<std::sync::RwLock<C>>,
    next_cursor: Option<C>,
) -> Option<String> {
    match next_cursor {
        Some(next_cursor) => {
            let next_cursor = Arc::new(std::sync::RwLock::new(next_cursor));

            let mut cursors_lock = cursors.write().unwrap();
            let mut id;

            loop {
                id = rand_b62(11);
                if !cursors_lock.contains_key(&id) {
                    break;
                }
            }

            let mut current_cursor = current_cursor.write().unwrap();

            current_cursor.set_next_cursor_id(id.clone());

            cursors_lock.insert(id.clone(), next_cursor);

            match current_cursor.prev_cursor_id() {
                Some(prev_cursor_id) => {
                    // if current cursor was accessed, we can drop previous one
                    cursors_lock.remove(prev_cursor_id);
                }
                None => {}
            }

            Some(id)
        }
        None => {
            let current_cursor = current_cursor.read().unwrap();

            match current_cursor.prev_cursor_id() {
                Some(prev_cursor_id) => {
                    let mut cursors_lock = cursors.write().unwrap();
                    cursors_lock.remove(prev_cursor_id);
                    None
                }
                None => None,
            }
        }
    }
}
