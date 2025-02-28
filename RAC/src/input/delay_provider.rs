use crate::logger::logger::{log_error, log_info};
use std::time::Duration;
use std::sync::Arc;

pub struct DelayProvider {
    delay_buffer: Vec<Duration>,
    current_index: usize,
}

impl DelayProvider {
    pub fn new() -> Self {
        let context = "DelayProvider::new";
        let mut provider = Self {
            delay_buffer: vec![Duration::ZERO; 512],
            current_index: 0,
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

    fn initialize_delay_buffer(&mut self) -> Result<(), String> {
        let mut rng = rand::rng();

        for delay in self.delay_buffer.iter_mut() {
            let ms = rand::Rng::random_range(&mut rng, 68.0..=73.0);

            let micro_adjust: i32 = rand::Rng::random_range(&mut rng, -150..=150);
            let final_delay = ms * 1000.0 + micro_adjust as f64;

            *delay = Duration::from_micros(final_delay.max(50000.0) as u64);
        }

        Ok(())
    }

    #[inline(always)]
    pub fn get_next_delay(&mut self) -> Duration {
        let delay = self.delay_buffer[self.current_index];
        self.current_index = (self.current_index + 1) & 511;
        delay
    }
}