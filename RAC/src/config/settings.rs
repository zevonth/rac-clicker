use crate::logger::logger::{log_error, log_info};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use crate::config::constants::defaults;

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub toggle_key: i32,
    pub target_process: String,
    pub adaptive_cpu_mode: bool,
    pub click_delay_micros: u64,
    pub delay_range_min: f64,
    pub delay_range_max: f64,
    pub random_deviation_min: i32,
    pub random_deviation_max: i32,
    pub keyboard_hold_mode: bool,
    pub max_cps: u8,
    pub burst_mode: bool,
    pub game_mode: String,
}

impl Settings {
    pub fn default_with_toggle_key(toggle_key: i32) -> Self {
        Self {
            toggle_key,
            target_process: defaults::TARGET_PROCESS.to_string(),
            adaptive_cpu_mode: defaults::ADAPTIVE_CPU_MODE,
            click_delay_micros: defaults::CLICK_DELAY_MICROS,
            delay_range_min: defaults::DELAY_RANGE_MIN,
            delay_range_max: defaults::DELAY_RANGE_MAX,
            random_deviation_min: defaults::RANDOM_DEVIATION_MIN,
            random_deviation_max: defaults::RANDOM_DEVIATION_MAX,
            keyboard_hold_mode: defaults::KEYBOARD_HOLD_MODE,
            max_cps: defaults::MAX_CPS,
            burst_mode: true,
            game_mode: "Combo".to_string(),
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
                                log_info("Attempting to load with default values for new fields", context);

                                let mut default_settings = Settings::default();

                                if let Ok(partial) = serde_json::from_str::<serde_json::Value>(&json) {
                                    if let Some(toggle_key) = partial.get("toggle_key").and_then(|v| v.as_i64()) {
                                        default_settings.toggle_key = toggle_key as i32;
                                    }
                                }

                                if let Err(save_err) = default_settings.save() {
                                    log_error(&format!("Failed to save updated settings: {}", save_err), context);
                                }

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