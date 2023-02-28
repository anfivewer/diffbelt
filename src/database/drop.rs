use crate::database::Database;

impl Drop for Database {
    fn drop(&mut self) {
        self.inner.on_database_drop();

        if let Some(sender) = self.stop_sender.take() {
            sender.send(true).unwrap_or(());
        }
    }
}
