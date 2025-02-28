use crate::input::click_strategy::{ClickStrategy, PostMessageClickStrategy};
use crate::input::delay_provider::DelayProvider;
use crate::input::thread_priority_manager::{ThreadPriorityManager, WindowsThreadPriorityManager};
use crate::input::handle::Handle;
use crate::input::thread_sync::{SyncManager, ThreadSyncManager};
use crate::input::window_finder::{ProcessWindowFinder, WindowFinder};
use crate::logger::logger::{log_error, log_info};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct WindowsClickService {
    sync_manager: Arc<ThreadSyncManager>,
    delay_provider: Arc<Mutex<DelayProvider>>,
    window_finder: Box<dyn WindowFinder + Send + Sync>,
    click_strategy: Box<dyn ClickStrategy + Send + Sync>,
    priority_manager: Box<dyn ThreadPriorityManager + Send + Sync>,
    window_handle: Arc<Mutex<Handle>>,
}

impl WindowsClickService {
    pub fn new(target_process: &str) -> Arc<Self> {
        let context = "WindowsClickService::new";

        let sync_manager = Arc::new(ThreadSyncManager::new());
        let window_finder = Box::new(ProcessWindowFinder::new(target_process));
        let click_strategy = Box::new(PostMessageClickStrategy);
        let priority_manager = Box::new(WindowsThreadPriorityManager::new());

        let service = Arc::new(Self {
            sync_manager,
            delay_provider: Arc::new(Mutex::new(DelayProvider::new())),
            window_finder,
            click_strategy,
            priority_manager,
            window_handle: Arc::new(Mutex::new(Handle::new())),
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

    fn update_window_handle(&self) {
        if let Some(hwnd) = self.window_finder.find_target_window() {
            let mut handle = self.window_handle.lock().unwrap();
            handle.set(hwnd);
        }
    }

    fn click_loop(&self) {
        let context = "WindowsClickService::click_loop";
        let mut last_click = Instant::now();
        let mut last_window_check = Instant::now();

        self.priority_manager.set_below_normal_priority();

        while !thread::panicking() {
            let is_enabled = self.sync_manager.wait_for_activation(Duration::from_millis(500));

            let now = Instant::now();
            let window_check_interval = if is_enabled {
                Duration::from_secs(1)
            } else {
                Duration::from_secs(3)
            };

            if now.duration_since(last_window_check) >= window_check_interval {
                self.update_window_handle();
                last_window_check = now;
            }

            if is_enabled {
                self.priority_manager.set_normal_priority();
            } else {
                self.priority_manager.set_below_normal_priority();
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let hwnd = {
                let handle = self.window_handle.lock().unwrap();
                handle.get()
            };

            if !hwnd.is_null() {
                self.click_strategy.perform_click(hwnd);

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
        self.sync_manager.toggle();
    }

    pub fn is_enabled(&self) -> bool {
        self.sync_manager.is_enabled()
    }
}