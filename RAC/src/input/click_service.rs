use crate::input::delay_provider::DelayProvider;
use crate::logger::logger::{log_error, log_info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority};
use windows::Win32::System::Threading::{THREAD_PRIORITY_BELOW_NORMAL, THREAD_PRIORITY_NORMAL};
use winapi::{
    shared::{minwindef::{DWORD, LPARAM}, windef::HWND},
    um::winuser::{EnumWindows, GetWindowThreadProcessId, PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP},
};
use std::ptr::null_mut;
use sysinfo::{ProcessesToUpdate, System};
use crate::input::handle::Handle;

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

struct ThreadSync {
    enabled: AtomicBool,
    mutex: Mutex<bool>,
    condvar: Condvar,
}

pub struct WindowsClickService {
    sync: Arc<ThreadSync>,
    pub(crate) delay_provider: Arc<Mutex<DelayProvider>>,
    target_process: String,
    hwnd: Arc<Mutex<Handle>>,
    system: Arc<Mutex<System>>,
}

impl WindowsClickService {
    pub fn new(target_process: &str) -> Arc<Self> {
        let context = "WindowsClickService::new";

        let sync = Arc::new(ThreadSync {
            enabled: AtomicBool::new(false),
            mutex: Mutex::new(false),
            condvar: Condvar::new(),
        });

        let service = Arc::new(Self {
            sync,
            delay_provider: Arc::new(Mutex::new(DelayProvider::new())),
            target_process: target_process.to_string(),
            hwnd: Arc::new(Mutex::new(Handle::new())),
            system: Arc::new(Mutex::new(System::new_all())),
        });

        let service_clone = service.clone();
        match thread::Builder::new()
            .name("ClickThread".to_string())
            .spawn(move || {
                service_clone.click_loop();
            }) {
            Ok(_) => {
                log_info("Click thread spawned successfully", context);
                service
            }
            Err(e) => {
                log_error(&format!("Failed to spawn click thread: {}", e), context);
                service
            }
        }
    }

    fn find_target_window(&self) -> Option<HWND> {
        let context = "WindowsClickService::find_target_window";

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
                    let mut hwnd_guard = self.hwnd.lock().unwrap();
                    hwnd_guard.set(data.hwnd);
                    return Some(data.hwnd);
                }

                let mut hwnd_guard = self.hwnd.lock().unwrap();
                hwnd_guard.set(null_mut());
                log_info(&format!("Lost target window for PID: {}", pid), context);
            }
        }

        None
    }

    fn click_loop(&self) {
        let context = "WindowsClickService::click_loop";
        let mut last_click = Instant::now();
        let mut last_window_check = Instant::now();

        unsafe {
            if let Err(e) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) {
                log_error(&format!("Failed to set initial thread priority: {:?}", e), context);
            }
        }

        while !thread::panicking() {
            let is_enabled = {
                let mut enabled = self.sync.mutex.lock().unwrap();
                if !*enabled && !self.sync.enabled.load(Ordering::Relaxed) {
                    let result = self.sync.condvar.wait_timeout(enabled, Duration::from_millis(500)).unwrap();
                    enabled = result.0;
                }
                *enabled
            };

            let now = Instant::now();
            let window_check_interval = if is_enabled {
                Duration::from_secs(1)
            } else {
                Duration::from_secs(3)
            };

            if now.duration_since(last_window_check) >= window_check_interval {
                self.find_target_window();
                last_window_check = now;
            }

            unsafe {
                if is_enabled {
                    if let Err(e) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_NORMAL) {
                        log_error(&format!("Failed to set normal thread priority: {:?}", e), context);
                    }
                } else {
                    if let Err(e) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) {
                        log_error(&format!("Failed to set below normal thread priority: {:?}", e), context);
                    }

                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
            }

            let hwnd = {
                let hwnd_guard = self.hwnd.lock().unwrap();
                hwnd_guard.get()
            };

            if !hwnd.is_null() {
                unsafe {
                    if let Err(_) = std::panic::catch_unwind(|| {
                        PostMessageA(hwnd, WM_LBUTTONDOWN, 0, 0);
                        thread::sleep(Duration::from_micros(900));
                        PostMessageA(hwnd, WM_LBUTTONUP, 0, 0);
                    }) {
                        log_error("Failed to execute window-specific mouse event", context);
                    }
                }

                let delay = {
                    let mut delay_provider = self.delay_provider.lock().unwrap();
                    delay_provider.get_next_delay()
                };

                let elapsed = last_click.elapsed();
                if elapsed < delay {
                    thread::sleep(delay.saturating_sub(elapsed));
                }
                last_click = Instant::now();
            } else {
                thread::sleep(Duration::from_millis(50));
            }
        }

        log_error("Click loop terminated due to thread panic", context);
    }

    pub fn toggle(&self) {
        let new_state = !self.sync.enabled.load(Ordering::Relaxed);
        self.sync.enabled.store(new_state, Ordering::Relaxed);

        let mut enabled = self.sync.mutex.lock().unwrap();
        *enabled = new_state;
        self.sync.condvar.notify_one();
    }

    pub fn is_enabled(&self) -> bool {
        self.sync.enabled.load(Ordering::Relaxed)
    }
}