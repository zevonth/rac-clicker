use crate::input::click_service::{ClickService, ClickServiceConfig};
use crate::menu::Menu;
use crate::validation::system_validator::SystemValidator;
#[cfg(target_os = "windows")]
#[cfg(not(debug_assertions))]
use debugoff;
use std::error::Error;
use std::io;
use std::sync::Arc;
use tokio;
use windows::core::{w, BOOL, PCSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Diagnostics::Debug::{CheckRemoteDebuggerPresent, IsDebuggerPresent};
use windows::Win32::System::Threading::{CreateMutexW, GetCurrentProcess};
use windows::Win32::UI::WindowsAndMessaging::FindWindowA;
use crate::input::click_executor::ClickExecutor;

pub mod config;
pub mod input;
pub mod menu;
pub mod validation;
mod logger;
mod auth;

pub struct ClickServiceMenu {
    click_service: Arc<ClickService>,
    click_executor: Arc<ClickExecutor>,
}

impl ClickServiceMenu {
    pub fn new(click_service: Arc<ClickService>, click_executor: Arc<ClickExecutor>) -> Self {
        Self {
            click_service,
            click_executor,
        }
    }
}

fn initialize_services() -> Result<(), String> {
    let validator = SystemValidator::new();
    let validation_result = validator.validate_system();
    if !validation_result.is_valid {
        return Err(validation_result.message.unwrap_or_else(|| "Unknown validation error".to_string()));
    }

    Ok(())
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
            let click_service = Arc::new(ClickService::new(ClickServiceConfig::default()));
            let click_executor = Arc::clone(&click_service.click_executor);
            let mut menu = Menu::new(Arc::clone(&click_service), click_executor);
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