use crate::input::delay_provider::DelayProvider;
use crate::logger::logger::{log_error, log_info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::{mouse_event, MOUSE_EVENT_FLAGS};

pub struct WindowsClickService {
    is_enabled: Arc<AtomicBool>,
    delay_provider: Arc<Mutex<DelayProvider>>,
}

impl WindowsClickService {
    pub fn new() -> Arc<Self> {
        let context = "WindowsClickService::new";
        let service = Arc::new(Self {
            is_enabled: Arc::new(AtomicBool::new(false)),
            delay_provider: Arc::new(Mutex::new(DelayProvider::new())),
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

    fn click_loop(&self) {
        let context = "WindowsClickService::click_loop";
        let mut delay_provider = self.delay_provider.lock().unwrap();
        let mut last_click = Instant::now();

        while !thread::panicking() {
            let is_enabled = self.is_enabled.load(Ordering::Relaxed);
            if is_enabled {
                unsafe {
                    if let Err(_) = std::panic::catch_unwind(|| {
                        mouse_event(MOUSE_EVENT_FLAGS(0x0002), 0, 0, 0, 0);
                        thread::sleep(Duration::from_micros(900));
                        mouse_event(MOUSE_EVENT_FLAGS(0x0004), 0, 0, 0, 0);
                    }) {
                        log_error("Failed to execute mouse event", context);
                        continue;
                    }
                }

                let delay = delay_provider.get_next_delay();
                let elapsed = last_click.elapsed();

                if elapsed < delay {
                    let sleep_time = delay.saturating_sub(elapsed);
                    thread::sleep(sleep_time);
                }

                last_click = Instant::now();
            } else {
                thread::sleep(Duration::from_millis(50));
            }
        }

        log_error("Click loop terminated due to thread panic", context);
    }

    pub fn toggle(&self) {
        let new_state = !self.is_enabled.load(Ordering::Relaxed);
        self.is_enabled.store(new_state, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled.load(Ordering::Relaxed)
    }
}