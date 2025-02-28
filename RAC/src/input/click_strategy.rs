use crate::logger::logger::log_error;
use std::thread;
use std::time::Duration;
use winapi::shared::windef::HWND;
use winapi::um::winuser::{PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP};

pub trait ClickStrategy {
    fn perform_click(&self, hwnd: HWND);
}

pub struct PostMessageClickStrategy;

impl ClickStrategy for PostMessageClickStrategy {
    fn perform_click(&self, hwnd: HWND) {
        let context = "PostMessageClickStrategy::perform_click";

        unsafe {
            if let Err(_) = std::panic::catch_unwind(|| {
                PostMessageA(hwnd, WM_LBUTTONDOWN, 0, 0);
                thread::sleep(Duration::from_micros(900));
                PostMessageA(hwnd, WM_LBUTTONUP, 0, 0);
            }) {
                log_error("Failed to execute window-specific mouse event", context);
            }
        }
    }
}