use crate::logger::logger::log_error;
use std::time::Duration;
use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority};
use windows::Win32::System::Threading::{THREAD_PRIORITY_BELOW_NORMAL, THREAD_PRIORITY_NORMAL, THREAD_PRIORITY_TIME_CRITICAL};

pub struct ThreadController {
    adaptive_mode: bool,
}

impl ThreadController {
    pub(crate) fn clone(&self) -> ThreadController {
        ThreadController {
            adaptive_mode: self.adaptive_mode,
        }
    }
}

impl ThreadController {
    pub fn new(adaptive_mode: bool) -> Self {
        Self { adaptive_mode }
    }

    pub fn set_adaptive_mode(&self, adaptive_mode: bool) {
        unsafe {
            let self_ptr = self as *const ThreadController as *mut ThreadController;
            (*self_ptr).adaptive_mode = adaptive_mode;
        }
    }

    pub fn set_active_priority(&self) {
        let context = "ThreadController::set_active_priority";
        unsafe {
            let priority = if self.adaptive_mode {
                THREAD_PRIORITY_NORMAL
            } else {
                THREAD_PRIORITY_TIME_CRITICAL
            };

            if let Err(e) = SetThreadPriority(GetCurrentThread(), priority) {
                log_error(&format!("Failed to set active thread priority: {:?}", e), context);
            }
        }
    }

    pub fn set_normal_priority(&self) {
        let context = "ThreadController::set_normal_priority";
        unsafe {
            if let Err(e) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_NORMAL) {
                log_error(&format!("Failed to set normal thread priority: {:?}", e), context);
            }
        }
    }

    pub fn set_idle_priority(&self) {
        let context = "ThreadController::set_idle_priority";
        unsafe {
            if let Err(e) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) {
                log_error(&format!("Failed to set idle thread priority: {:?}", e), context);
            }
        }
    }

    pub fn smart_sleep(&self, duration: Duration) {
        if self.adaptive_mode && duration > Duration::from_millis(5) {
            let chunk_size = Duration::from_millis(2);
            let mut remaining = duration;

            while remaining > chunk_size {
                std::thread::sleep(chunk_size);
                remaining -= chunk_size;
                std::thread::yield_now();
            }

            if remaining > Duration::ZERO {
                std::thread::sleep(remaining);
            }
        } else {
            std::thread::sleep(duration);
        }
    }
}