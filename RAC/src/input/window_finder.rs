use crate::input::handle::Handle;
use crate::logger::logger::log_info;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use sysinfo::{ProcessesToUpdate, System};
use winapi::{
    shared::{minwindef::{DWORD, LPARAM}, windef::HWND},
    um::winuser::{EnumWindows, GetWindowThreadProcessId},
};

struct FindWindowData {
    pid: DWORD,
    hwnd: HWND,
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> i32 {
    let data = &mut *(lparam as *mut FindWindowData);
    let mut process_id: DWORD = 0;
    GetWindowThreadProcessId(hwnd, &mut process_id);

    if process_id == data.pid {
        data.hwnd = hwnd;
        return 0;
    }
    1
}

pub struct WindowFinder {
    target_process: String,
    system: Arc<Mutex<System>>,
}

impl WindowFinder {
    pub fn new(target_process: &str) -> Self {
        Self {
            target_process: target_process.to_string(),
            system: Arc::new(Mutex::new(System::new_all())),
        }
    }

    pub fn update_target_process(&self, new_target_process: &str) -> bool {
        let context = "WindowFinder::update_target_process";
        if self.target_process == new_target_process {
            return false;
        }
        
        unsafe {
            let self_ptr = self as *const WindowFinder as *mut WindowFinder;
            (*self_ptr).target_process = new_target_process.to_string();
        }
        
        log_info(&format!("Updated target process to: {}", new_target_process), context);
        true
    }

    pub fn find_target_window(&self, hwnd_handle: &Arc<Mutex<Handle>>) -> Option<HWND> {
        let context = "WindowFinder::find_target_window";

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
            let mut data = FindWindowData {
                pid,
                hwnd: null_mut(),
            };

            unsafe {
                EnumWindows(Some(enum_windows_callback), &mut data as *mut _ as LPARAM);
                if !data.hwnd.is_null() {
                    let mut hwnd_guard = hwnd_handle.lock().unwrap();
                    hwnd_guard.set(data.hwnd);
                    return Some(data.hwnd);
                }

                let mut hwnd_guard = hwnd_handle.lock().unwrap();
                hwnd_guard.set(null_mut());
                log_info(&format!("Lost target window for PID: {}", pid), context);
            }
        }

        None
    }
}