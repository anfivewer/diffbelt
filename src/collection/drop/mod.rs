use crate::collection::Collection;

impl Drop for Collection {
    fn drop(&mut self) {
        if let Some(sender) = self.drop_sender.take() {
            sender.send(()).unwrap_or(());
        }
    }
}
