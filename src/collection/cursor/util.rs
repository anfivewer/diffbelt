use crate::util::base62::rand_b62;
use std::collections::HashMap;
use std::sync::Arc;

pub trait BaseCursor {
    fn get_prev_cursor_id(&self) -> Option<&str>;
}

pub fn save_next_cursor<C: BaseCursor>(
    cursors: &std::sync::RwLock<HashMap<String, Arc<C>>>,
    current_cursor: &C,
    next_cursor: Option<C>,
) -> Option<String> {
    match next_cursor {
        Some(next_cursor) => {
            let next_cursor = Arc::new(next_cursor);

            let mut cursors_lock = cursors.write().unwrap();
            let mut id;

            loop {
                id = rand_b62(11);
                if !cursors_lock.contains_key(&id) {
                    break;
                }
            }

            cursors_lock.insert(id.clone(), next_cursor.clone());

            match current_cursor.get_prev_cursor_id() {
                Some(prev_cursor_id) => {
                    // if current cursor was accessed, we can drop previous one
                    cursors_lock.remove(prev_cursor_id);
                }
                None => {}
            }

            Some(id)
        }
        None => match current_cursor.get_prev_cursor_id() {
            Some(prev_cursor_id) => {
                let mut cursors_lock = cursors.write().unwrap();
                cursors_lock.remove(prev_cursor_id);
                None
            }
            None => None,
        },
    }
}
