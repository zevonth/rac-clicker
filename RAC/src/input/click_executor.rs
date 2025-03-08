use crate::input::thread_controller::ThreadController;
use crate::config::settings::Settings;
use crate::config::constants::defaults;
use crate::logger::logger::{log_error, log_info};
use rand::Rng;
use std::time::Duration;
use std::sync::Mutex;
use winapi::{
    shared::windef::HWND,
    um::winuser::{PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP},
};
use winapi::um::winuser::{MK_LBUTTON};

#[derive(Debug, Clone)]
pub enum GameMode {
    Combo,
    Default
}
pub struct ClickExecutor {
    thread_controller: ThreadController,
    click_delay_micros: u64,
    pub(crate) game_mode: Mutex<GameMode>,
    max_cps: u8,
}

impl ClickExecutor {
    pub fn new(thread_controller: ThreadController) -> Self {
        let settings = Settings::load().unwrap_or_else(|_| Settings::default());

        let game_mode = match settings.game_mode.as_str() {
            "Combo" => GameMode::Combo,
            _ => GameMode::Default,
        };

        Self {
            thread_controller,
            click_delay_micros: settings.click_delay_micros,
            game_mode: Mutex::new(game_mode),
            max_cps: defaults::MAX_CPS
        }
    }

    pub fn update_delay(&self, click_delay_micros: u64) {
        if self.click_delay_micros == click_delay_micros {
            return;
        }

        unsafe {
            let self_ptr = self as *const ClickExecutor as *mut ClickExecutor;
            (*self_ptr).click_delay_micros = click_delay_micros;
        }
    }

    pub fn set_game_mode(&self, mode: GameMode) {
        let context = "ClickExecutor::set_game_mode";

        if let Ok(mut game_mode) = self.game_mode.lock() {
            *game_mode = mode.clone();
        }

        log_info(&format!("Game mode set to: {:?}", mode), context);
    }

    pub fn get_game_mode(&self) -> GameMode {
        self.game_mode.lock().unwrap().clone()
    }

    pub fn execute_click(&self, hwnd: HWND) -> bool {
        if hwnd.is_null() {
            return false;
        }

        let context = "ClickExecutor::execute_click";

        unsafe {
            if let Err(_) = std::panic::catch_unwind(|| {
                let mut rng = rand::rng();

                PostMessageA(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, 0);

                let down_time = rng.random_range(5000..6000);
                self.thread_controller.smart_sleep(Duration::from_micros(down_time));

                PostMessageA(hwnd, WM_LBUTTONUP, 0, 0);

                let post_click_pause = rng.random_range(200..1000);
                self.thread_controller.smart_sleep(Duration::from_micros(post_click_pause));

                let base_delay = self.get_mode_specific_delay(&mut rng);
                let remaining_delay = base_delay.saturating_sub(
                    Duration::from_micros(down_time + post_click_pause)
                );
                self.thread_controller.smart_sleep(remaining_delay);

            }) {
                log_error("Failed to execute window-specific mouse event", context);
                return false;
            }
        }

        true
    }
    fn get_mode_specific_delay(&self, rng: &mut impl Rng) -> Duration {
        let base_delay = self.click_delay_micros;

        let game_mode = self.game_mode.lock().unwrap();

        let adjusted_delay = match *game_mode {
            GameMode::Default => {
                let jitter = rng.random_range(-2000..2000);
                base_delay.saturating_add_signed(jitter)
            },
            GameMode::Combo => {
                let jitter = rng.random_range(-1000..5000); // 7000 30000
                base_delay.saturating_add_signed(jitter)
            }
        };

        let min_cps_delay = 1_000_000 / self.max_cps as u64;
        let min_down_up = 15000;

        Duration::from_micros(adjusted_delay.max(min_down_up).max(min_cps_delay))
    }
}