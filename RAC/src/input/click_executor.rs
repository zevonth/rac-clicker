use crate::input::thread_controller::ThreadController;
use crate::config::settings::Settings;
use crate::logger::logger::log_error;
use std::time::Duration;
use winapi::{
    shared::windef::HWND,
    um::winuser::{PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP},
};

pub struct ClickExecutor {
    thread_controller: ThreadController,
    click_delay_micros: u64,
}

impl ClickExecutor {
    pub fn new(thread_controller: ThreadController) -> Self {
        let settings = Settings::load().unwrap_or_else(|_| Settings::default());

        Self {
            thread_controller,
            click_delay_micros: settings.click_delay_micros,
        }
    }
    
    pub fn update_delay(&self, click_delay_micros: u64) {
        if self.click_delay_micros == click_delay_micros {
            return;
        }
        
        unsafe {
            let self_ptr = self as *const ClickExecutor as *mut ClickExecutor;
            (*self_ptr).click_delay_micros = click_delay_micros;
        }
    }

    pub fn execute_click(&self, hwnd: HWND) -> bool {
        if hwnd.is_null() {
            return false;
        }

        let context = "ClickExecutor::execute_click";

        unsafe {
            if let Err(_) = std::panic::catch_unwind(|| {
                PostMessageA(hwnd, WM_LBUTTONDOWN, 0, 0);

                self.thread_controller.smart_sleep(Duration::from_micros(self.click_delay_micros));

                PostMessageA(hwnd, WM_LBUTTONUP, 0, 0);
            }) {
                log_error("Failed to execute window-specific mouse event", context);
                return false;
            }
        }

        true
    }
}