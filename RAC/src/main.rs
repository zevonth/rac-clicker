#[cfg(target_os = "windows")]
#[cfg(not(debug_assertions))]
use debugoff;
use crate::auth::license_checker::LicenseChecker;
use crate::auth::license_validator::LicenseValidator;
use crate::input::click_service::WindowsClickService;
use crate::menu::Menu;
use crate::validation::system_validator::SystemValidator;
use std::error::Error;
use std::sync::Arc;
use std::io;
use tokio;
use windows::core::{w, PCSTR};
use windows::Win32::Foundation::{GetLastError, BOOL, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Diagnostics::Debug::{CheckRemoteDebuggerPresent, IsDebuggerPresent};
use windows::Win32::System::Threading::{CreateMutexW, GetCurrentProcess};
use windows::Win32::UI::WindowsAndMessaging::FindWindowA;
use crate::auth::license_keys::{XOR_KEY, PROTECTED_PUBLIC, PROTECTED_ENCRYPTION};

pub mod config;
pub mod input;
pub mod menu;
pub mod validation;
mod logger;
mod auth;


pub struct ClickServiceMenu {
    click_service: Arc<WindowsClickService>,
}

impl ClickServiceMenu {
    pub fn new(click_service: Arc<WindowsClickService>) -> Self {
        Self { click_service }
    }
}

fn initialize_services() -> Result<(), String> {
    let validator = SystemValidator::new();
    let validation_result = validator.validate_system();
    if !validation_result.is_valid {
        return Err(validation_result.message.unwrap_or_else(|| "Unknown validation error".to_string()));
    }

    let license_validator = LicenseValidator::new(Vec::from(XOR_KEY), Vec::from(PROTECTED_PUBLIC), Vec::from(PROTECTED_ENCRYPTION))
        .map_err(|e| format!("License initialization error: {}", e))?;

    println!("\nYour Machine ID: {}", license_validator.get_current_machine_id());
    println!("\nPlace your license file in this directory:");
    println!("{}", license_validator.get_license_dir());
    println!("\nLicense filename should be: {}.license", license_validator.get_current_machine_id());

    match license_validator.validate_license() {
        Ok(true) => {
            println!("\nLicense is valid!");
            Ok(())
        },
        Ok(false) => Err("License is invalid or expired!".to_string()),
        Err(e) => Err(format!("License validation error: {}", e)),
    }
}

fn check_single_instance() -> bool {
    unsafe {
        let mutex_name = w!("Global\\RACApplicationMutex");
        CreateMutexW(None, true, mutex_name).expect("TODO: panic message");
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

#[cfg(target_os = "windows")]
fn check_debugger() -> bool {
    use windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
    unsafe { IsDebuggerPresent().as_bool() }
}

pub fn check_debugger_presence() -> bool {
    unsafe {
        if IsDebuggerPresent().as_bool() {
            return true;
        }

        let mut is_debugged = BOOL(0);
        CheckRemoteDebuggerPresent(GetCurrentProcess(), &mut is_debugged).expect("TODO: panic message");
        if is_debugged.as_bool() {
            return true;
        }

        let debuggers = [
            "x64dbg",
            "ida",
            "ida64",
            "ollydbg",
            "cheatengine-x86_64",
            "HTTPDebuggerUI",
            "ProcessHacker",
            "dnSpy",
            "cheatengine-i386",
            "ReClass.NET",
            "Wireshark",
            "Fiddler",
        ];

        for debugger in debuggers {
            let window = FindWindowA(
                PCSTR::null(),
                PCSTR::from_raw(debugger.as_ptr()),
            );

            if let Ok(handle) = window {
                if !handle.is_invalid() {
                    return true;
                }
            }
        }

        false
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if !check_single_instance() {
        eprintln!("Application is already running!");
        println!("\nPress Enter to exit...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        std::process::exit(1);
    }

    #[cfg(target_os = "windows")]
    #[cfg(not(debug_assertions))]
    debugoff::multi_ptraceme_or_die();

    if check_debugger_presence() {
        std::process::exit(1);
    }

    if check_debugger() {
        std::process::exit(1);
    }

    match initialize_services() {
        Ok(()) => {
            let click_service = WindowsClickService::new();
            let license_validator = LicenseValidator::new(Vec::from(XOR_KEY), Vec::from(PROTECTED_PUBLIC), Vec::from(PROTECTED_ENCRYPTION))?;
            let license_checker = LicenseChecker::new(license_validator);

            license_checker.start_checking().await;

            let mut menu = Menu::new(click_service);
            menu.show_main_menu();
        }
        Err(error_message) => {
            eprintln!("System validation failed: {}", error_message);
            println!("\nPress Enter to exit...");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            std::process::exit(1);
        }
    }

    Ok(())
}