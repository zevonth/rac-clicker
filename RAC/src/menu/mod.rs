use crate::auth::license_validator::LicenseValidator;
use crate::config::settings::Settings;
use crate::input::click_service::WindowsClickService;
use crate::auth::license_keys::{PROTECTED_ENCRYPTION, PROTECTED_PUBLIC, XOR_KEY};
use crate::logger::logger::{log_error, log_info};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use windows::core::PCSTR;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextA};
use windows::Win32::System::Console::SetConsoleTitleA;

pub struct Menu {
    click_service: Arc<WindowsClickService>,
    toggle_key: i32,
}

impl Menu {
    pub fn new(click_service: Arc<WindowsClickService>) -> Self {
        let context = "Menu::new";

        let toggle_key = match Settings::load() {
            Ok(settings) => {
                log_info("Loaded existing hotkey configuration", context);
                settings.toggle_key
            },
            Err(_) => {
                log_info("No existing hotkey configuration found", context);
                0
            }
        };

        let menu = Self {
            click_service,
            toggle_key,
        };

        log_info("Menu initialized successfully", context);
        menu
    }

    fn clear_console(&self) {
        print!("\x1B[2J\x1B[3J\x1B[1;1H");
        if let Err(_e) = io::stdout().flush() {
            log_error("Failed to clear console", "Menu::clear_console");
        }
    }

    pub fn show_main_menu(&mut self) {
        let context = "Menu::show_main_menu";
        loop {
            unsafe {
                SetConsoleTitleA(PCSTR::from_raw("RAC Menu\0".as_ptr())).expect("TODO: panic message");
            }

            self.clear_console();

            println!("=== RAC Menu ===");
            println!("1. Configure Hotkey");
            println!("2. Start RAC");
            println!("3. Show Current Settings");
            println!("4. Exit");
            print!("\nSelect option: ");

            if let Err(e) = io::stdout().flush() {
                log_error(&format!("Failed to flush stdout: {}", e), context);
            }

            let mut choice = String::new();
            if let Err(e) = io::stdin().read_line(&mut choice) {
                log_error(&format!("Failed to read user input: {}", e), context);
                continue;
            }

            match choice.trim() {
                "1" => self.configure_hotkey(),
                "2" => self.start_auto_clicker(),
                "3" => {
                    self.show_current_settings();
                    println!("\nPress Enter to continue...");
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                },
                "4" => std::process::exit(0),
                _ => {
                    log_error("Invalid menu option selected", context);
                    println!("\nInvalid option! Press Enter to continue...");
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                }
            }
        }
    }

    fn configure_hotkey(&mut self) {
        let context = "Menu::configure_hotkey";

        self.clear_console();
        println!("=== Hotkey Configuration ===");
        println!("1. Configure Mouse Button");
        println!("2. Configure Keyboard Key");
        println!("3. Back to Main Menu");
        print!("\nSelect option: ");

        if let Err(e) = io::stdout().flush() {
            log_error(&format!("Failed to flush stdout: {}", e), context);
            return;
        }

        let mut choice = String::new();
        if let Err(e) = io::stdin().read_line(&mut choice) {
            log_error(&format!("Failed to read user input: {}", e), context);
            return;
        }

        match choice.trim() {
            "1" => self.configure_mouse_hotkey(),
            "2" => self.configure_keyboard_hotkey(),
            "3" => return,
            _ => {
                log_error("Invalid hotkey configuration option selected", context);
                println!("\nInvalid option! Press Enter to continue...");
                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);
            }
        }
    }

    fn configure_keyboard_hotkey(&mut self) {
        let context = "Menu::configure_keyboard_hotkey";

        self.clear_console();
        println!("=== Keyboard Hotkey Configuration ===");
        println!("\nPress any key (A-Z) to set as hotkey...");

        if let Err(e) = io::stdout().flush() {
            log_error(&format!("Failed to flush stdout: {}", e), context);
            return;
        }

        let mut input = [0u8; 1];
        if let Err(e) = io::stdin().read_exact(&mut input) {
            log_error(&format!("Failed to read keyboard input: {}", e), context);
            return;
        }

        let key = input[0];
        let virtual_key = match key as char {
            'a'..='z' => key.to_ascii_uppercase() as i32,
            'A'..='Z' => key as i32,
            _ => {
                println!("\nInvalid key! Please press a letter key (A-Z)...");
                return;
            }
        };

        self.toggle_key = virtual_key;
        let settings = Settings { toggle_key: self.toggle_key };
        if let Err(e) = settings.save() {
            log_error(&format!("Failed to save settings: {}", e), context);
        } else {
            println!("\nHotkey successfully set to: {}", Self::get_key_name(virtual_key));
            println!("To change the hotkey, return to the main menu and configure again.");
            println!("\nPress Enter to continue...");

            let mut _input = String::new();
            if let Err(e) = io::stdin().read_line(&mut _input) {
                log_error(&format!("Failed to read continue prompt: {}", e), context);
            }
        }
    }

    fn configure_mouse_hotkey(&mut self) {
        let context = "Menu::configure_mouse_hotkey";
        println!("\nPress any mouse button to set as hotkey...");

        if let Err(e) = io::stdout().flush() {
            log_error(&format!("Failed to flush stdout: {}", e), context);
            return;
        }

        let mut mouse_key = 0;
        while mouse_key == 0 {
            for key in 1..=12 {
                unsafe {
                    let state = GetAsyncKeyState(key);
                    if (state as u16 & 0x8000) != 0 {
                        mouse_key = key;
                        break;
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        self.toggle_key = mouse_key;
        let settings = Settings { toggle_key: self.toggle_key };
        if let Err(e) = settings.save() {
            log_error(&format!("Failed to save settings: {}", e), context);
        } else {
            println!("\nHotkey successfully set to: {}", Self::get_key_name(mouse_key));
            println!("To change the hotkey, return to the main menu and configure again.");
            println!("\nPress Enter to continue...");

            let mut _input = String::new();
            if let Err(e) = io::stdin().read_line(&mut _input) {
                log_error(&format!("Failed to read continue prompt: {}", e), context);
            }
        }
    }

    fn show_current_settings(&self) {
        self.clear_console();

        println!("=== Current Settings ===");
        println!("Toggle Hotkey: {}", Self::get_key_name(self.toggle_key));
        /*
        if let Ok(license_validator) = LicenseValidator::new(Vec::from(XOR_KEY), Vec::from(PROTECTED_PUBLIC), Vec::from(PROTECTED_ENCRYPTION)) {
            println!("Machine ID: {}", license_validator.get_current_machine_id());

            match license_validator.get_license_info() {
                Ok(license_info) => {
                    let now = OffsetDateTime::now_utc().unix_timestamp();
                    let remaining = license_info.expires_at - now;

                    if remaining > 0 {
                        let days = remaining / 86400;
                        let hours = (remaining % 86400) / 3600;
                        let minutes = (remaining % 3600) / 60;
                        let seconds = remaining % 60;
                        println!(
                            "License expires in: {} days {} hours {} minutes {} seconds",
                            days, hours, minutes, seconds
                        );
                    } else {
                        println!("License has expired!");
                    }
                }
                Err(e) => println!("Could not read license info: {}", e),
            }
        }
        */
    }

    fn start_auto_clicker(&self) {
        if self.toggle_key == 0 {
            println!("Please configure hotkey first!");
            return;
        }

        self.clear_console();
        println!("RAC Started! Press {} to toggle.", Self::get_key_name(self.toggle_key));

        self.run_main_loop();
    }

    fn run_main_loop(&self) {
        let mut last_toggle = Instant::now();
        let toggle_cooldown = Duration::from_millis(300);
        let mut was_rmb_held = false;
        let mut was_enabled_before_rmb = false;

        let loop_sleep_duration = Duration::from_millis(10);

        println!("Press Ctrl + Q to return to main menu (only works when focused on this window)");
        println!("Hold Right Mouse Button to temporarily pause clicking");

        loop {
            unsafe {
                static mut LAST_WINDOW_CHECK: Option<Instant> = None;
                let current_time = Instant::now();

                let mut is_our_window = false;
                if let Some(last_check) = LAST_WINDOW_CHECK {
                    if current_time.duration_since(last_check) >= Duration::from_millis(500) {
                        let foreground_window = GetForegroundWindow();
                        let mut title = [0u8; 256];
                        let len = GetWindowTextA(foreground_window, &mut title);
                        let window_title = String::from_utf8_lossy(&title[..len as usize]);
                        is_our_window = window_title.trim() == "RAC Menu";
                        LAST_WINDOW_CHECK = Some(current_time);
                    }
                } else {
                    LAST_WINDOW_CHECK = Some(current_time);
                }

                if is_our_window &&
                    (GetAsyncKeyState(0x11) as u16 & 0x8000) != 0 &&
                    (GetAsyncKeyState(0x51) as u16 & 0x8000) != 0 {
                    if self.click_service.is_enabled() {
                        self.click_service.toggle();
                    }
                    return;
                }

                let rmb_pressed = (GetAsyncKeyState(0x02) as u16 & 0x8000) != 0;

                if (GetAsyncKeyState(self.toggle_key) as u16 & 0x8000) != 0 {
                    if current_time.duration_since(last_toggle) > toggle_cooldown {
                        self.click_service.toggle();
                        last_toggle = current_time;
                        was_enabled_before_rmb = false;
                        was_rmb_held = false;
                    }
                }

                if rmb_pressed {
                    if self.click_service.is_enabled() {
                        was_enabled_before_rmb = true;
                        self.click_service.toggle();
                    }
                    was_rmb_held = true;
                } else if was_rmb_held && !self.click_service.is_enabled() && was_enabled_before_rmb {
                    self.click_service.toggle();
                    was_enabled_before_rmb = false;
                    was_rmb_held = false;
                }
            }

            thread::sleep(loop_sleep_duration);
        }
    }

    fn get_key_name(key: i32) -> String {
        match key {
            0x01 => "Left Mouse Button".to_string(),
            0x02 => "Right Mouse Button".to_string(),
            0x04 => "Middle Mouse Button".to_string(),
            0x05 => "X1 Mouse Button".to_string(),
            0x06 => "X2 Mouse Button".to_string(),

            0x07 => "Mouse Button 7".to_string(),
            0x08 => "Mouse Button 8".to_string(),
            0x09 => "Mouse Button 9".to_string(),
            0x0A => "Mouse Button 10".to_string(),
            0x0B => "Mouse Button 11".to_string(),
            0x0C => "Mouse Button 12".to_string(),

            0x41..=0x5A => format!("Key {}", (key as u8 as char)),

            _ => format!("Unknown Key (0x{:02X})", key),
        }
    }

}
