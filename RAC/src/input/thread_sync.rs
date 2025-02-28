use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub trait SyncManager {
    fn is_enabled(&self) -> bool;
    fn wait_for_activation(&self, timeout: Duration) -> bool;
    fn toggle(&self);
}

pub struct ThreadSyncManager {
    enabled: AtomicBool,
    mutex: Mutex<bool>,
    condvar: Condvar,
}

impl ThreadSyncManager {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            mutex: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }
}

impl SyncManager for ThreadSyncManager {
    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    fn wait_for_activation(&self, timeout: Duration) -> bool {
        let mut enabled = self.mutex.lock().unwrap();
        if !*enabled && !self.enabled.load(Ordering::Relaxed) {
            let result = self.condvar.wait_timeout(enabled, timeout).unwrap();
            enabled = result.0;
        }
        *enabled
    }

    fn toggle(&self) {
        let new_state = !self.enabled.load(Ordering::Relaxed);
        self.enabled.store(new_state, Ordering::Relaxed);

        let mut enabled = self.mutex.lock().unwrap();
        *enabled = new_state;
        self.condvar.notify_one();
    }
}