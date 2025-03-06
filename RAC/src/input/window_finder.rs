use crate::input::handle::Handle;
use crate::logger::logger::{log_info};
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use sysinfo::{ProcessesToUpdate, System};
use winapi::{
    shared::{minwindef::{DWORD, LPARAM}, windef::HWND},
    um::winuser::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible},
};
use winapi::um::winuser::GetWindowTextW;

struct FindWindowData {
    pid: DWORD,
    hwnd: HWND,
    window_count: u32,
    require_visibility: bool,
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> i32 {
    let data = &mut *(lparam as *mut FindWindowData);
    let mut process_id: DWORD = 0;
    GetWindowThreadProcessId(hwnd, &mut process_id);

    if process_id == data.pid {
        let is_visible = IsWindowVisible(hwnd) != 0;

        let mut title: [u16; 512] = [0; 512];
        let title_len = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
        let window_title = if title_len > 0 {
            String::from_utf16_lossy(&title[0..title_len as usize])
        } else {
            String::from("[No Title]")
        };

        log_info(&format!("Found window for PID {}: HWND={:?}, Visible={}, Title='{}'",
                           data.pid, hwnd, is_visible, window_title),
                  "enum_windows_callback");

        if !data.require_visibility || is_visible {
            data.hwnd = hwnd;
            data.window_count += 1;
            return 1;
        }
    }
    1
}


pub struct WindowFinder {
    target_process: String,
    system: Arc<Mutex<System>>,
    last_found_pid: Option<DWORD>,
    require_visibility: bool,
}

impl WindowFinder {
    pub fn new(target_process: &str) -> Self {
        Self {
            target_process: target_process.to_string(),
            system: Arc::new(Mutex::new(System::new_all())),
            last_found_pid: None,
            require_visibility: true,
        }
    }

    pub fn set_require_visibility(&mut self, require: bool) {
        self.require_visibility = require;
        log_info(&format!("Window visibility requirement set to: {}", require),
                 "WindowFinder::set_require_visibility");
    }

    pub fn update_target_process(&self, new_target_process: &str) -> bool {
        let context = "WindowFinder::update_target_process";
        if self.target_process == new_target_process {
            return false;
        }

        unsafe {
            let self_ptr = self as *const WindowFinder as *mut WindowFinder;
            (*self_ptr).target_process = new_target_process.to_string();
            (*self_ptr).last_found_pid = None;
        }

        log_info(&format!("Updated target process to: {}", new_target_process), context);
        true
    }

    pub fn find_target_window(&self, hwnd_handle: &Arc<Mutex<Handle>>) -> Option<HWND> {
        let context = "WindowFinder::find_target_window";

        if let Some(pid) = self.last_found_pid {
            if let Some(hwnd) = self.find_window_for_pid(pid) {
                let mut hwnd_guard = hwnd_handle.lock().unwrap();
                hwnd_guard.set(hwnd);
                return Some(hwnd);
            }
        }

        let mut sys = self.system.lock().unwrap();
        sys.refresh_processes(ProcessesToUpdate::All, false);

        let mut target_pid: Option<DWORD> = None;
        for (pid, process) in sys.processes() {
            let name = process.name().to_string_lossy();
            if name.to_lowercase() == self.target_process.to_lowercase() {
                target_pid = Some(pid.as_u32());
                break;
            }
        }

        drop(sys);

        if let Some(pid) = target_pid {
            unsafe {
                let self_ptr = self as *const WindowFinder as *mut WindowFinder;
                (*self_ptr).last_found_pid = Some(pid);
            }

            if let Some(hwnd) = self.find_window_for_pid(pid) {
                let mut hwnd_guard = hwnd_handle.lock().unwrap();
                hwnd_guard.set(hwnd);
                return Some(hwnd);
            } else {
                log_info(&format!("Found process '{}' (PID: {}) but it has no visible windows",
                                  self.target_process, pid), context);
            }
        } else {
            log_info(&format!("Process '{}' not found", self.target_process), context);
        }

        let mut hwnd_guard = hwnd_handle.lock().unwrap();
        hwnd_guard.set(null_mut());
        None
    }

    fn find_window_for_pid(&self, pid: DWORD) -> Option<HWND> {
        let context = "WindowFinder::find_window_for_pid";
        log_info(&format!("Looking for {} windows for process PID: {}",
                          if self.require_visibility { "visible" } else { "any" }, pid), context);

        let mut data = FindWindowData {
            pid,
            hwnd: null_mut(),
            window_count: 0,
            require_visibility: self.require_visibility,
        };

        unsafe {
            EnumWindows(Some(enum_windows_callback), &mut data as *mut _ as LPARAM);

            if !data.hwnd.is_null() {
                log_info(&format!("Found {} window(s) for process PID: {}",
                                  data.window_count, pid), context);
                return Some(data.hwnd);
            } else if data.window_count > 0 {
                log_info(&format!("Found {} windows for PID: {} but none matched visibility requirements",
                                  data.window_count, pid), context);
            } else {
                log_info(&format!("No windows found for PID: {}", pid), context);
            }
        }

        None
    }
}