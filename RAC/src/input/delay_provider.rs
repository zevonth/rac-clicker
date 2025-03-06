use crate::logger::logger::{log_error, log_info};
use crate::config::settings::Settings;
use rand::Rng;
use std::time::Duration;

pub struct DelayProvider {
    delay_buffer: Vec<Duration>,
    current_index: usize,
    delay_range_min: f64,
    delay_range_max: f64,
    random_deviation_min: i32,
    random_deviation_max: i32,
    pub(crate) burst_mode: bool,
    burst_counter: u8,
}

impl DelayProvider {
    pub fn new() -> Self {
        let context = "DelayProvider::new";

        let settings = Settings::load().unwrap_or_else(|_| Settings::default());

        let mut provider = Self {
            delay_buffer: vec![Duration::ZERO; 512],
            current_index: 0,
            delay_range_min: settings.delay_range_min,
            delay_range_max: settings.delay_range_max,
            random_deviation_min: settings.random_deviation_min,
            random_deviation_max: settings.random_deviation_max,
            burst_mode: settings.burst_mode,
            burst_counter: 0,
        };

        match provider.initialize_delay_buffer() {
            Ok(_) => {
                log_info("Delay buffer initialized successfully", context);
                provider
            }
            Err(e) => {
                log_error(&format!("Failed to initialize delay buffer: {}", e), context);
                provider
            }
        }
    }

    pub fn toggle_burst_mode(&mut self) -> bool {
        self.burst_mode = !self.burst_mode;
        self.burst_counter = 0;
        self.burst_mode
    }

    pub fn update_settings(&mut self,
                           delay_range_min: f64,
                           delay_range_max: f64,
                           random_deviation_min: i32,
                           random_deviation_max: i32) {
        let context = "DelayProvider::update_settings";

        let settings_changed =
            self.delay_range_min != delay_range_min ||
                self.delay_range_max != delay_range_max ||
                self.random_deviation_min != random_deviation_min ||
                self.random_deviation_max != random_deviation_max;

        if !settings_changed {
            return;
        }

        self.delay_range_min = delay_range_min;
        self.delay_range_max = delay_range_max;
        self.random_deviation_min = random_deviation_min;
        self.random_deviation_max = random_deviation_max;

        if let Err(e) = self.initialize_delay_buffer() {
            log_error(&format!("Failed to reinitialize delay buffer: {}", e), context);
        } else {
            log_info("Delay buffer reinitialized with new settings", context);
        }
    }

    fn initialize_delay_buffer(&mut self) -> Result<(), String> {
        let mut rng = rand::rng();
        for delay in self.delay_buffer.iter_mut() {
            let ms = rng.random_range(self.delay_range_min..=self.delay_range_max);
            *delay = Duration::from_micros((ms * 1000.0) as u64);
        }
        Ok(())
    }

    pub fn get_next_delay(&mut self) -> Duration {
        let mut rng = rand::rng();

        if self.burst_mode && self.burst_counter < 1 {
            self.burst_counter += 1;
            return Duration::from_micros(rng.random_range(58000..62000));
        } else if self.burst_mode {
            self.burst_counter = 0;
        }

        let base_delay = self.delay_buffer[self.current_index];
        self.current_index = (self.current_index + 1) & 511;

        let micro_adjust: i32 = rng.random_range(-500..500);

        let final_delay = if micro_adjust < 0 {
            base_delay.saturating_sub(Duration::from_micros(-micro_adjust as u64))
        } else {
            base_delay.saturating_add(Duration::from_micros(micro_adjust as u64))
        };

        if final_delay < Duration::from_micros(45000) {
            return Duration::from_micros(45000);
        }

        final_delay
    }
}