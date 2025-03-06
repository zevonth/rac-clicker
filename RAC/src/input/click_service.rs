use crate::input::click_executor::ClickExecutor;
use crate::input::delay_provider::DelayProvider;
use crate::input::handle::Handle;
use crate::input::sync_controller::SyncController;
use crate::input::thread_controller::ThreadController;
use crate::input::window_finder::WindowFinder;
use crate::logger::logger::{log_error, log_info};
use crate::config::settings::Settings;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct ClickServiceConfig {
    pub target_process: String,
    pub window_check_active_interval: Duration,
    pub window_check_idle_interval: Duration,
    pub adaptive_cpu_mode: bool,
}

impl Default for ClickServiceConfig {
    fn default() -> Self {
        let settings = Settings::load().unwrap_or_else(|_| Settings::default());

        Self {
            target_process: settings.target_process,
            window_check_active_interval: Duration::from_secs(1),
            window_check_idle_interval: Duration::from_secs(3),
            adaptive_cpu_mode: settings.adaptive_cpu_mode,
        }
    }
}

pub struct ClickService {
    sync_controller: Arc<SyncController>,
    pub(crate) delay_provider: Arc<Mutex<DelayProvider>>,
    hwnd: Arc<Mutex<Handle>>,
    window_finder: Arc<WindowFinder>,
    pub(crate) click_executor: Arc<ClickExecutor>,
    thread_controller: Arc<ThreadController>,
    config: ClickServiceConfig,
    settings: Arc<Mutex<Settings>>,
    window_finder_running: Arc<AtomicBool>,
}

impl ClickService {
    pub fn new(config: ClickServiceConfig) -> Arc<Self> {
        let context = "ClickService::new";

        let settings = Settings::load().unwrap_or_else(|_| Settings::default());

        let thread_controller = Arc::new(ThreadController::new(config.adaptive_cpu_mode));

        let service = Arc::new(Self {
            sync_controller: Arc::new(SyncController::new()),
            delay_provider: Arc::new(Mutex::new(DelayProvider::new())),
            hwnd: Arc::new(Mutex::new(Handle::new())),
            window_finder: Arc::new(WindowFinder::new(&config.target_process)),
            click_executor: Arc::new(ClickExecutor::new((*thread_controller).clone())),
            thread_controller,
            config,
            settings: Arc::new(Mutex::new(settings)),
            window_finder_running: Arc::new(AtomicBool::new(true)),
        });

        let service_clone = service.clone();
        match thread::Builder::new()
            .name("WindowFinderThread".to_string())
            .spawn(move || {
                service_clone.window_finder_loop();
            }) {
            Ok(_) => {
                log_info("Window finder thread spawned successfully", context);
            }
            Err(e) => {
                log_error(&format!("Failed to spawn window finder thread: {}", e), context);
            }
        }

        let service_clone = service.clone();
        match thread::Builder::new()
            .name("SettingsSyncThread".to_string())
            .spawn(move || {
                service_clone.settings_sync_loop();
            }) {
            Ok(_) => {
                log_info("Settings synchronization thread spawned successfully", context);
            }
            Err(e) => {
                log_error(&format!("Failed to spawn settings sync thread: {}", e), context);
            }
        }

        let service_clone = service.clone();
        match thread::Builder::new()
            .name("ClickThread".to_string())
            .spawn(move || {
                service_clone.click_loop();
            }) {
            Ok(_) => {
                log_info("Click thread spawned successfully", context);
            }
            Err(e) => {
                log_error(&format!("Failed to spawn click thread: {}", e), context);
            }
        }

        service
    }

    fn window_finder_loop(&self) {
        let context = "ClickService::window_finder_loop";
        log_info("Window finder thread started", context);

        self.thread_controller.set_idle_priority();

        while !thread::panicking() && self.window_finder_running.load(Ordering::SeqCst) {
            let check_interval = if self.is_enabled() {
                self.config.window_check_active_interval
            } else {
                self.config.window_check_idle_interval
            };

            self.window_finder.find_target_window(&self.hwnd);

            thread::sleep(check_interval);
        }

        log_info("Window finder thread terminated", context);
    }

    fn settings_sync_loop(&self) {
        let context = "ClickService::settings_sync_loop";
        log_info("Settings synchronization thread started", context);

        self.thread_controller.set_idle_priority();

        while !thread::panicking() {
            self.check_and_update_settings();

            thread::sleep(Duration::from_secs(5));
        }

        log_error("Settings sync loop terminated due to thread panic", context);
    }

    fn check_and_update_settings(&self) {
        let context = "ClickService::check_and_update_settings";

        match Settings::load() {
            Ok(new_settings) => {
                let target_process;
                let target_process_new = new_settings.target_process.clone();
                let adaptive_cpu_mode;
                let click_delay_micros;
                let delay_range_min;
                let delay_range_max;
                let random_deviation_min;
                let random_deviation_max;
                
                {
                    let current_settings = self.settings.lock().unwrap();
                    target_process = current_settings.target_process.clone();
                    adaptive_cpu_mode = current_settings.adaptive_cpu_mode;
                    click_delay_micros = current_settings.click_delay_micros;
                    delay_range_min = current_settings.delay_range_min;
                    delay_range_max = current_settings.delay_range_max;
                    random_deviation_min = current_settings.random_deviation_min;
                    random_deviation_max = current_settings.random_deviation_max;
                }

                let target_process_changed = target_process != target_process_new;
                let adaptive_cpu_mode_changed = adaptive_cpu_mode != new_settings.adaptive_cpu_mode;
                let click_delay_changed = click_delay_micros != new_settings.click_delay_micros;
                let delay_range_changed = 
                    delay_range_min != new_settings.delay_range_min || 
                    delay_range_max != new_settings.delay_range_max;
                let deviation_changed = 
                    random_deviation_min != new_settings.random_deviation_min || 
                    random_deviation_max != new_settings.random_deviation_max;

                {
                    let mut current_settings = self.settings.lock().unwrap();
                    *current_settings = new_settings;
                }

                if target_process_changed {
                    log_info(&format!("Target process updated to: {}", target_process_new), context);
                    let _ = self.window_finder.update_target_process(&target_process_new);
                }
                
                if adaptive_cpu_mode_changed {
                    log_info(&format!("Adaptive CPU mode updated to: {}", if adaptive_cpu_mode { "disabled" } else { "enabled" }), context);
                    self.thread_controller.set_adaptive_mode(!adaptive_cpu_mode);
                }
                
                if click_delay_changed || delay_range_changed || deviation_changed {
                    log_info("Click timing parameters updated", context);

                    if delay_range_changed || deviation_changed {
                        if let Ok(mut delay_provider) = self.delay_provider.lock() {
                            delay_provider.update_settings(
                                delay_range_min,
                                delay_range_max,
                                random_deviation_min,
                                random_deviation_max
                            );
                        }
                    }

                    if click_delay_changed {
                        self.click_executor.update_delay(click_delay_micros);
                    }
                }
            },
            Err(e) => {
                log_error(&format!("Failed to reload settings: {}", e), context);
            }
        }
    }

    fn click_loop(&self) {
        let context = "ClickService::click_loop";
        let mut last_click = Instant::now();
        let mut consecutive_failures = 0;
        let mut was_previously_disabled = true;

        self.thread_controller.set_active_priority();

        while !thread::panicking() {
            let is_enabled = self.sync_controller.wait_for_signal(Duration::from_millis(100));

            if is_enabled && was_previously_disabled {
                last_click = Instant::now();
                was_previously_disabled = false;
            } else if !is_enabled {
                was_previously_disabled = true;
            }

            if is_enabled {
                self.thread_controller.set_active_priority();
            } else {
                self.thread_controller.set_normal_priority();
                self.thread_controller.smart_sleep(Duration::from_millis(50));
                consecutive_failures = 0;
                continue;
            }

            let hwnd = {
                let hwnd_guard = self.hwnd.lock().unwrap();
                hwnd_guard.get()
            };

            if self.click_executor.execute_click(hwnd) {
                consecutive_failures = 0;

                let delay = {
                    let mut delay_provider = self.delay_provider.lock().unwrap();
                    delay_provider.get_next_delay()
                };

                let elapsed = last_click.elapsed();
                if elapsed < delay {
                    self.thread_controller.smart_sleep(delay.saturating_sub(elapsed));
                }
                last_click = Instant::now();
            } else {
                consecutive_failures += 1;

                if consecutive_failures >= 3 {
                    log_info("Multiple click failures detected, continuing with next cycle", context);
                    consecutive_failures = 0;
                }

                self.thread_controller.smart_sleep(Duration::from_millis(20));
            }
        }

        self.window_finder_running.store(false, Ordering::SeqCst);
        log_error("Click loop terminated due to thread panic", context);
    }

    pub fn toggle(&self) -> bool {
        self.sync_controller.toggle()
    }

    pub fn is_enabled(&self) -> bool {
        self.sync_controller.is_enabled()
    }

    pub fn force_enable_clicking(&self) -> bool {
        if self.is_enabled() {
            return true;
        }

        log_info("Forcing click service to enable state", "ClickService::force_enable_clicking");
        self.sync_controller.force_enable()
    }

    pub fn force_disable_clicking(&self) -> bool {
        if !self.is_enabled() {
            return true;
        }

        log_info("Forcing click service to disable state", "ClickService::force_disable_clicking");

        if self.sync_controller.is_enabled() {
            self.sync_controller.toggle();
        }

        true
    }
}