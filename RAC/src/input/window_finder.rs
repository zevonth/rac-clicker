use crate::logger::logger::{log_info};
use std::ptr::null_mut;
use std::sync::Arc;
use winapi::shared::{minwindef::{DWORD, LPARAM}, windef::HWND};
use winapi::um::winuser::{EnumWindows, GetWindowThreadProcessId};
use sysinfo::{ProcessesToUpdate, System};

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

pub trait WindowFinder {
    fn find_target_window(&self) -> Option<HWND>;
}

pub struct ProcessWindowFinder {
    target_process: String,
    system: Arc<std::sync::Mutex<System>>,
    context: &'static str,
}

impl ProcessWindowFinder {
    pub fn new(target_process: &str) -> Self {
        Self {
            target_process: target_process.to_string(),
            system: Arc::new(std::sync::Mutex::new(System::new_all())),
            context: "ProcessWindowFinder",
        }
    }
}

impl WindowFinder for ProcessWindowFinder {
    fn find_target_window(&self) -> Option<HWND> {
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
                    return Some(data.hwnd);
                }
                log_info(&format!("Lost target window for PID: {}", pid), self.context);
            }
        }

        None
    }
}