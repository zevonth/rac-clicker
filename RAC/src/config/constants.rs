pub mod defaults {
    pub const TOGGLE_KEY: i32 = 0;
    pub const TARGET_PROCESS: &str = "craftrise-x64.exe";
    pub const ADAPTIVE_CPU_MODE: bool = false;
    pub const CLICK_DELAY_MICROS: u64 = 75;
    pub const DELAY_RANGE_MIN: f64 = 69.5;
    pub const DELAY_RANGE_MAX: f64 = 70.5;
    pub const RANDOM_DEVIATION_MIN: i32 = -50;
    pub const RANDOM_DEVIATION_MAX: i32 = 50;
    pub const KEYBOARD_HOLD_MODE: bool = false;
    pub const MAX_CPS: u8 = 16;
    pub const MIN_DELAY_FOR_DEFAULT_CPS: u64 = 62500;
}