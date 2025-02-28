use crate::logger::logger::{log_error, log_info};
use crate::validation::validation_result::ValidationResult;
use std::path::PathBuf;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

pub struct SystemRequirements {
    minimum_windows_version: i32,
    required_directories: Vec<PathBuf>,
}

impl Default for SystemRequirements {
    fn default() -> Self {
        let context = "SystemRequirements::default";
        let rac_dir = dirs::data_local_dir().unwrap().join("RAC");
        let logs_path = rac_dir.join("logs.txt");

        if !rac_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&rac_dir) {
                log_error(&format!("Failed to create RAC directory: {}", e), context);
            }
        }

        if !logs_path.exists() {
            if let Err(e) = std::fs::write(&logs_path, "") {
                log_error(&format!("Failed to create logs file: {}", e), context);
            }
        }

        Self {
            minimum_windows_version: 10,
            required_directories: vec![rac_dir],
        }
    }
}

pub struct SystemValidator {
    requirements: SystemRequirements,
}

impl SystemValidator {
    pub fn new() -> Self {
        let context = "SystemValidator::new";
        log_info("Initializing system validator", context);
        Self {
            requirements: SystemRequirements::default(),
        }
    }

    pub fn validate_system(&self) -> ValidationResult {
        let context = "SystemValidator::validate_system";
        let validations = [
            self.validate_operating_system(),
            self.validate_windows_version(),
            self.validate_directory_permissions(),
            self.validate_mouse_access(),
        ];

        for result in validations {
            if !result.is_valid {
                if let Some(msg) = &result.message {
                    log_error(msg, context);
                }
                return result;
            }
        }

        log_info("System validation completed successfully", context);
        ValidationResult::with_message(true, "System validation successful")
    }

    fn validate_operating_system(&self) -> ValidationResult {
        let context = "SystemValidator::validate_operating_system";
        if !cfg!(windows) {
            let error_msg = format!("Unsupported operating system. Required: Windows, Current: {}", std::env::consts::OS);
            log_error(&error_msg, context);
            return ValidationResult::with_message(false, error_msg);
        }
        ValidationResult::new(true)
    }

    fn validate_windows_version(&self) -> ValidationResult {
        let context = "SystemValidator::validate_windows_version";
        let version = os_info::get();
        let version_str = version.version().to_string();
        let major_version: i32 = match version_str.split('.').next().unwrap().parse() {
            Ok(v) => v,
            Err(e) => {
                let error_msg = format!("Failed to parse Windows version: {}", e);
                log_error(&error_msg, context);
                return ValidationResult::with_message(false, error_msg);
            }
        };

        if major_version < self.requirements.minimum_windows_version {
            let error_msg = format!(
                "Unsupported Windows version. Required: {}, Current: {}",
                self.requirements.minimum_windows_version,
                major_version
            );
            log_error(&error_msg, context);
            return ValidationResult::with_message(false, error_msg);
        }
        ValidationResult::new(true)
    }

    fn validate_directory_permissions(&self) -> ValidationResult {
        let context = "SystemValidator::validate_directory_permissions";
        for dir in &self.requirements.required_directories {
            if let Err(e) = std::fs::create_dir_all(dir) {
                let error_msg = format!("Directory permission check failed for: {}", dir.display());
                log_error(&format!("{}: {}", error_msg, e), context);
                return ValidationResult::with_error(false, error_msg, e);
            }

            let test_file = dir.join(format!("test_{}.tmp", uuid::Uuid::new_v4()));
            if let Err(e) = std::fs::write(&test_file, "test") {
                let error_msg = format!("Failed to write test file in: {}", dir.display());
                log_error(&format!("{}: {}", error_msg, e), context);
                return ValidationResult::with_error(false, error_msg, e);
            }
            let _ = std::fs::remove_file(test_file);
        }
        ValidationResult::new(true)
    }

    fn validate_mouse_access(&self) -> ValidationResult {
        let context = "SystemValidator::validate_mouse_access";
        unsafe {
            let mut point = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut point as *mut _).is_err() {
                let error_msg = "Failed to access mouse controls";
                log_error(error_msg, context);
                return ValidationResult::with_message(false, error_msg);
            }
            ValidationResult::new(true)
        }
    }
}