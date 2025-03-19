use crate::logger::logger::{log_error, log_info};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use serde::de::Error;
use crate::config::constants::defaults;
use tokio::fs;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub toggle_key: i32,
    pub target_process: String,
    pub adaptive_cpu_mode: bool,

    pub left_click_delay_micros: u64,
    pub right_click_delay_micros: u64,
    pub left_random_deviation_min: i32,
    pub left_random_deviation_max: i32,
    pub right_random_deviation_min: i32,
    pub right_random_deviation_max: i32,
    pub keyboard_hold_mode: bool,
    pub left_max_cps: u8,
    pub right_max_cps: u8,
    pub left_game_mode: String,
    pub right_game_mode: String,
    pub click_mode: String,

    #[serde(skip_serializing, default)]
    pub click_delay_micros: u64,
    #[serde(skip_serializing, default)]
    pub delay_range_min: f64,
    #[serde(skip_serializing, default)]
    pub delay_range_max: f64,
    #[serde(skip_serializing, default)]
    pub left_delay_range_min: f64,
    #[serde(skip_serializing, default)]
    pub left_delay_range_max: f64,
    #[serde(skip_serializing, default)]
    pub right_delay_range_min: f64,
    #[serde(skip_serializing, default)]
    pub right_delay_range_max: f64,
    #[serde(skip_serializing, default)]
    pub random_deviation_min: i32,
    #[serde(skip_serializing, default)]
    pub random_deviation_max: i32,
    #[serde(skip_serializing, default)]
    pub burst_mode: bool,
    #[serde(skip_serializing, default)]
    pub left_burst_mode: bool,
    #[serde(skip_serializing, default)]
    pub right_burst_mode: bool,
    #[serde(skip_serializing, default)]
    pub game_mode: String,
    pub max_cps: u8,
}

impl Settings {
    pub fn default_with_toggle_key(toggle_key: i32) -> Self {
        Self {
            toggle_key,
            target_process: defaults::TARGET_PROCESS.to_string(),
            adaptive_cpu_mode: defaults::ADAPTIVE_CPU_MODE,
            left_click_delay_micros: defaults::CLICK_DELAY_MICROS,
            right_click_delay_micros: defaults::CLICK_DELAY_MICROS,
            left_random_deviation_min: defaults::RANDOM_DEVIATION_MIN,
            left_random_deviation_max: defaults::RANDOM_DEVIATION_MAX,
            right_random_deviation_min: defaults::RANDOM_DEVIATION_MIN,
            right_random_deviation_max: defaults::RANDOM_DEVIATION_MAX,
            keyboard_hold_mode: defaults::KEYBOARD_HOLD_MODE,
            left_max_cps: defaults::LEFT_MAX_CPS,
            right_max_cps: defaults::RIGHT_MAX_CPS,
            left_game_mode: "Combo".to_string(),
            right_game_mode: "Combo".to_string(),
            click_mode: "LeftClick".to_string(),
            click_delay_micros: defaults::CLICK_DELAY_MICROS,
            delay_range_min: defaults::DELAY_RANGE_MIN,
            delay_range_max: defaults::DELAY_RANGE_MAX,
            left_delay_range_min: defaults::DELAY_RANGE_MIN,
            left_delay_range_max: defaults::DELAY_RANGE_MAX,
            right_delay_range_min: defaults::DELAY_RANGE_MIN,
            right_delay_range_max: defaults::DELAY_RANGE_MAX,
            random_deviation_min: defaults::RANDOM_DEVIATION_MIN,
            random_deviation_max: defaults::RANDOM_DEVIATION_MAX,
            burst_mode: true,
            left_burst_mode: true,
            right_burst_mode: true,
            game_mode: "Combo".to_string(),
            max_cps: 15,
        }
    }

    pub fn default() -> Self {
        Self::default_with_toggle_key(defaults::TOGGLE_KEY)
    }

    fn get_settings_path() -> io::Result<PathBuf> {
        let local_app_data = dirs::data_local_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find AppData/Local directory"))?;

        let settings_dir = local_app_data.join("RAC");
        if !settings_dir.exists() {
            std::fs::create_dir_all(&settings_dir)?;
        }

        Ok(settings_dir.join("settings.json"))
    }

    pub fn save(&self) -> io::Result<()> {
        let context = "Settings::save";
        match Self::get_settings_path() {
            Ok(settings_path) => {
                match serde_json::to_string(self) {
                    Ok(json) => {
                        if let Err(e) = std::fs::write(&settings_path, json) {
                            log_error(&format!("Failed to write settings file: {}", e), context);
                            return Err(e);
                        }
                        log_info("Settings saved successfully", context);
                        Ok(())
                    }
                    Err(e) => {
                        log_error(&format!("Failed to serialize settings: {}", e), context);
                        Err(io::Error::new(io::ErrorKind::Other, e))
                    }
                }
            }
            Err(e) => {
                log_error(&format!("Failed to get settings path: {}", e), context);
                Err(e)
            }
        }
    }

    pub fn load() -> io::Result<Self> {
        let context = "Settings::load";
        match Self::get_settings_path() {
            Ok(settings_path) => {
                if !settings_path.exists() {
                    let default_settings = Settings::default();
                    log_info("Created default settings", context);
                    return Ok(default_settings);
                }

                match std::fs::read_to_string(&settings_path) {
                    Ok(json) => {
                        match serde_json::from_str(&json) {
                            Ok(settings) => {
                                log_info("Settings loaded successfully", context);
                                Ok(settings)
                            }
                            Err(e) => {
                                log_error(&format!("Failed to parse settings JSON: {}", e), context);
                                log_info("Trying to recover partial settings", context);

                                let mut default_settings = Settings::default();

                                if let Ok(partial) = serde_json::from_str::<serde_json::Value>(&json) {
                                    if let Some(toggle_key) = partial.get("toggle_key").and_then(|v| v.as_i64()) {
                                        default_settings.toggle_key = toggle_key as i32;
                                    }

                                    if let Some(left_max_cps) = partial.get("left_max_cps").and_then(|v| v.as_u64()) {
                                        default_settings.left_max_cps = left_max_cps as u8;
                                    }

                                    if let Some(right_max_cps) = partial.get("right_max_cps").and_then(|v| v.as_u64()) {
                                        default_settings.right_max_cps = right_max_cps as u8;
                                    }

                                    if let Some(left_game_mode) = partial.get("left_game_mode").and_then(|v| v.as_str()) {
                                        default_settings.left_game_mode = left_game_mode.to_string();
                                    }

                                    if let Some(right_game_mode) = partial.get("right_game_mode").and_then(|v| v.as_str()) {
                                        default_settings.right_game_mode = right_game_mode.to_string();
                                    }
                                }

                                log_info("Recovered partial settings, but not auto-saving", context);

                                Ok(default_settings)
                            }
                        }
                    }
                    Err(e) => {
                        log_error(&format!("Failed to read settings file: {}", e), context);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                log_error(&format!("Failed to get settings path: {}", e), context);
                Err(e)
            }
        }
    }
}