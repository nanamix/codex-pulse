use std::sync::atomic::{AtomicU8, Ordering};
/// 0: running, 1: child cleanup in progress, 2: cleanup completed.
#[derive(Default)]
pub struct ExitGate(AtomicU8);
impl ExitGate {
    pub fn request_cleanup(&self) -> bool {
        self.0
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
    pub fn can_exit(&self) -> bool {
        self.0.load(Ordering::SeqCst) == 2
    }
    pub fn finish(&self) {
        self.0.store(2, Ordering::SeqCst);
    }
}
