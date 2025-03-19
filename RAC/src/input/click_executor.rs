use crate::input::thread_controller::ThreadController;
use crate::config::settings::Settings;
use crate::logger::logger::{log_error, log_info};
use rand::Rng;
use std::time::Duration;
use std::sync::atomic::{AtomicU8, AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use winapi::{
    shared::windef::HWND,
    um::winuser::{PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP},
};
use winapi::um::winuser::{MK_LBUTTON, MK_RBUTTON};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseButton {
    Left,
    Right
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameMode {
    Combo,
    Default
}

pub struct ClickExecutor {
    thread_controller: ThreadController,
    left_game_mode: Arc<Mutex<GameMode>>,
    right_game_mode: Arc<Mutex<GameMode>>,
    left_max_cps: AtomicU8,
    right_max_cps: AtomicU8,
    left_click_delay_micros: AtomicUsize,
    right_click_delay_micros: AtomicUsize,
    active: AtomicBool,
    current_button: Mutex<MouseButton>,
}

impl ClickExecutor {
    pub fn new(thread_controller: ThreadController) -> Self {
        let settings = Settings::load().unwrap_or_else(|_| Settings::default());

        let left_mode = match settings.left_game_mode.as_str() {
            "Combo" => GameMode::Combo,
            _ => GameMode::Default,
        };
        
        let right_mode = match settings.right_game_mode.as_str() {
            "Combo" => GameMode::Combo,
            _ => GameMode::Default,
        };

        Self {
            thread_controller,
            left_game_mode: Arc::new(Mutex::new(left_mode)),
            right_game_mode: Arc::new(Mutex::new(right_mode)),
            left_max_cps: AtomicU8::new(settings.left_max_cps),
            right_max_cps: AtomicU8::new(settings.right_max_cps),
            left_click_delay_micros: AtomicUsize::new(settings.left_click_delay_micros as usize),
            right_click_delay_micros: AtomicUsize::new(settings.right_click_delay_micros as usize),
            active: AtomicBool::new(true),
            current_button: Mutex::new(MouseButton::Left),
        }
    }

    pub fn update_delay(&self, click_delay_micros: u64) {
        match *self.current_button.lock().unwrap() {
            MouseButton::Left => {
                self.left_click_delay_micros.store(click_delay_micros as usize, Ordering::SeqCst);
            },
            MouseButton::Right => {
                self.right_click_delay_micros.store(click_delay_micros as usize, Ordering::SeqCst);
            }
        }
    }

    pub fn set_left_max_cps(&self, max_cps: u8) {
        self.left_max_cps.store(max_cps, Ordering::SeqCst);
    }
    
    pub fn set_right_max_cps(&self, max_cps: u8) {
        self.right_max_cps.store(max_cps, Ordering::SeqCst);
    }

    pub fn set_max_cps(&self, max_cps: u8) {
        match *self.current_button.lock().unwrap() {
            MouseButton::Left => self.set_left_max_cps(max_cps),
            MouseButton::Right => self.set_right_max_cps(max_cps),
        }
    }

    pub fn set_left_game_mode(&self, mode: GameMode) {
        if let Ok(mut game_mode) = self.left_game_mode.lock() {
            *game_mode = mode;
        }
    }
    
    pub fn set_right_game_mode(&self, mode: GameMode) {
        if let Ok(mut game_mode) = self.right_game_mode.lock() {
            *game_mode = mode;
        }
    }

    pub fn set_game_mode(&self, mode: GameMode) {
        match *self.current_button.lock().unwrap() {
            MouseButton::Left => self.set_left_game_mode(mode),
            MouseButton::Right => self.set_right_game_mode(mode),
        }
    }
    
    pub fn get_game_mode(&self) -> GameMode {
        match *self.current_button.lock().unwrap() {
            MouseButton::Left => *self.left_game_mode.lock().unwrap(),
            MouseButton::Right => *self.right_game_mode.lock().unwrap(),
        }
    }

    pub fn set_mouse_button(&self, button: MouseButton) {
        if let Ok(mut current) = self.current_button.lock() {
            *current = button;
        }
    }

    pub fn execute_click(&self, hwnd: HWND) -> bool {
        if hwnd.is_null() || !self.active.load(Ordering::SeqCst) {
            return false;
        }

        let context = "ClickExecutor::execute_click";
        let button = match self.current_button.lock() {
            Ok(button) => *button,
            Err(e) => {
                log_error(&format!("Failed to lock current_button mutex: {}", e), context);
                return false;
            }
        };

        let (down_msg, up_msg, flags, max_cps, game_mode, _click_delay) = match button {
            MouseButton::Left => {
                (
                    WM_LBUTTONDOWN, 
                    WM_LBUTTONUP, 
                    MK_LBUTTON,
                    self.left_max_cps.load(Ordering::SeqCst),
                    *self.left_game_mode.lock().unwrap(),
                    self.left_click_delay_micros.load(Ordering::SeqCst) as u64
                )
            },
            MouseButton::Right => {
                (
                    WM_RBUTTONDOWN, 
                    WM_RBUTTONUP, 
                    MK_RBUTTON,
                    self.right_max_cps.load(Ordering::SeqCst),
                    *self.right_game_mode.lock().unwrap(),
                    self.right_click_delay_micros.load(Ordering::SeqCst) as u64
                )
            }
        };

        let cps_delay = if max_cps == 0 { 1_000_000 } else { 1_000_000 / max_cps as u64 };

        unsafe {
            if let Err(_) = std::panic::catch_unwind(|| {
                let mut rng = rand::rng();

                PostMessageA(hwnd, down_msg, flags, 0);

                let down_time = 1; // 0.25ms
                self.thread_controller.smart_sleep(Duration::from_micros(down_time));

                PostMessageA(hwnd, up_msg, 0, 0);

                let mut adjusted_delay = cps_delay.saturating_sub(down_time);

                if game_mode == GameMode::Combo {
                    #[allow(deprecated)]
                    let jitter = rng.gen_range(-500..=500);
                    
                    adjusted_delay = adjusted_delay.saturating_add_signed(jitter);

                    if adjusted_delay < cps_delay.saturating_sub(down_time) {
                        adjusted_delay = cps_delay.saturating_sub(down_time);
                    }
                }

                self.thread_controller.smart_sleep(Duration::from_micros(adjusted_delay));
            }) {
                log_error("Failed to execute mouse event", context);
                return false;
            }
        }

        true
    }

    pub fn get_current_max_cps(&self) -> u8 {
        match *self.current_button.lock().unwrap() {
            MouseButton::Left => self.left_max_cps.load(Ordering::SeqCst),
            MouseButton::Right => self.right_max_cps.load(Ordering::SeqCst),
        }
    }

    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::SeqCst);
    }

    pub fn force_right_cps(&self, cps: u8) {
        self.right_max_cps.store(cps, Ordering::SeqCst);
        log_info(&format!("Right click CPS forced to: {}", cps), "ClickExecutor::force_right_cps");
    }
}