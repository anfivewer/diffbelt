use crate::collection::Collection;
#[cfg(feature = "debug_prints")]
use crate::util::debug_print::debug_print;

impl Drop for Collection {
    fn drop(&mut self) {
        #[cfg(feature = "debug_prints")]
        debug_print(format!("Drop Collection {}", self.name).as_str());

        if let Some(sender) = self.drop_sender.take() {
            sender.send(()).unwrap_or(());
        }
    }
}
