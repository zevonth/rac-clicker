use crate::logger::logger::{log_error, log_info};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub toggle_key: i32,
}

impl Settings {
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
                    let default_settings = Settings { toggle_key: 0 };
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
                                Err(io::Error::new(io::ErrorKind::Other, e))
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