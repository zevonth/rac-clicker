use crate::logger::logger::log_error;
use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority};
use windows::Win32::System::Threading::{THREAD_PRIORITY_BELOW_NORMAL, THREAD_PRIORITY_NORMAL};

pub trait ThreadPriorityManager {
    fn set_normal_priority(&self);
    fn set_below_normal_priority(&self);
}

pub struct WindowsThreadPriorityManager {
    context: &'static str,
}

impl WindowsThreadPriorityManager {
    pub fn new() -> Self {
        Self { context: "WindowsThreadPriorityManager" }
    }
}

impl ThreadPriorityManager for WindowsThreadPriorityManager {
    fn set_normal_priority(&self) {
        unsafe {
            if let Err(e) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_NORMAL) {
                log_error(&format!("Failed to set normal thread priority: {:?}", e), self.context);
            }
        }
    }

    fn set_below_normal_priority(&self) {
        unsafe {
            if let Err(e) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) {
                log_error(&format!("Failed to set below normal thread priority: {:?}", e), self.context);
            }
        }
    }
}