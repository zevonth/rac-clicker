use crate::logger::logger::{log_error, log_info};
use rand::Rng;
use std::time::Duration;

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
            let ms = rng.random_range(69.5..=70.5);
            *delay = Duration::from_micros((ms * 1000.0) as u64);
        }
        Ok(())
    }

    #[inline(always)]
    pub fn get_next_delay(&mut self) -> Duration {
        let mut rng = rand::rng();
        let base_delay = self.delay_buffer[self.current_index];
        self.current_index = (self.current_index + 1) & 511;

        let micro_adjust: i32 = rng.random_range(-70..=80);
        if micro_adjust < 0 {
            base_delay.saturating_sub(Duration::from_micros(-micro_adjust as u64))
        } else {
            base_delay.saturating_add(Duration::from_micros(micro_adjust as u64))
        }
    }
}
