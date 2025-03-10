use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::Span;

pub struct RequestContext {
    pub is_flatbuffers_: Arc<AtomicBool>,
    pub span: Span,
}

impl RequestContext {
    pub fn is_flatbuffers(&self) -> bool {
        self.is_flatbuffers_.load(Ordering::Relaxed)
    }

    pub fn set_flatbuffers(&self) {
        self.is_flatbuffers_.store(true, Ordering::Relaxed);
    }
}
