use crate::config::settings::Settings;
use crate::input::click_service::ClickService;
use crate::input::click_executor::{ClickExecutor, GameMode};
use crate::logger::logger::{log_error, log_info};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use windows::core::PCSTR;
use windows::Win32::System::Console::SetConsoleTitleA;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextA};

#[derive(Clone, Copy, PartialEq)]
enum ToggleMode {
    MouseHold,
    KeyboardHold,
}

pub struct Menu {
    click_service: Arc<ClickService>,
    click_executor: Arc<ClickExecutor>,
    toggle_key: i32,
    toggle_mode: ToggleMode,
}

impl Menu {
    pub fn new(click_service: Arc<ClickService>, mut click_executor: Arc<ClickExecutor>) -> Self {
        let context = "Menu::new";

        let settings = match Settings::load() {
            Ok(s) => {
                log_info("Loaded existing configuration", context);

                if let Ok(mut delay_provider) = click_service.delay_provider.lock() {
                    if delay_provider.burst_mode != s.burst_mode {
                        delay_provider.toggle_burst_mode();
                    }
                }

                click_executor.set_game_mode(GameMode::Combo);

                s
            },
            Err(_) => {
                log_info("No existing configuration found", context);
                Settings::default()
            }
        };


        let menu = Self {
            click_service,
            click_executor,
            toggle_key: settings.toggle_key,
            toggle_mode: if settings.keyboard_hold_mode { ToggleMode::KeyboardHold } else { ToggleMode::MouseHold },
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

    fn configure_toggle_mode(&mut self) {
        let context = "Menu::configure_toggle_mode";

        self.clear_console();
        println!("=== Toggle Mode Configuration ===");
        println!("Select how you want to activate clicking:");
        println!("1. Mouse Hold Mode (Default) - Press toggle key to enable, then HOLD LEFT MOUSE BUTTON to click");
        println!("2. Keyboard Hold Mode - HOLD TOGGLE KEY to click");
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
            "1" => {
                self.toggle_mode = ToggleMode::MouseHold;
                let mut settings = match Settings::load() {
                    Ok(mut s) => {
                        s.keyboard_hold_mode = false;
                        s
                    },
                    Err(_) => {
                        let mut s = Settings::default();
                        s.keyboard_hold_mode = false;
                        s
                    }
                };

                if let Err(e) = settings.save() {
                    log_error(&format!("Failed to save settings: {}", e), context);
                    println!("Failed to save settings! Press Enter to continue...");
                } else {
                    println!("Mouse Hold Mode enabled! Press Enter to continue...");
                }
                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);
            },
            "2" => {
                self.toggle_mode = ToggleMode::KeyboardHold;
                let mut settings = match Settings::load() {
                    Ok(mut s) => {
                        s.keyboard_hold_mode = true;
                        s
                    },
                    Err(_) => {
                        let mut s = Settings::default();
                        s.keyboard_hold_mode = true;
                        s
                    }
                };

                if let Err(e) = settings.save() {
                    log_error(&format!("Failed to save settings: {}", e), context);
                    println!("Failed to save settings! Press Enter to continue...");
                } else {
                    println!("Keyboard Hold Mode enabled! Press Enter to continue...");
                }
                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);
            },
            "3" => return,
            _ => {
                log_error("Invalid toggle mode option selected", context);
                println!("\nInvalid option! Press Enter to continue...");
                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);
            }
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
            println!("4. Configure Advanced Settings");
            println!("5. Configure Toggle Mode");
            println!("6. Exit");
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
                "4" => self.configure_advanced_settings(),
                "5" => self.configure_toggle_mode(),
                "6" => self.perform_clean_exit(),
                _ => {
                    log_error("Invalid menu option selected", context);
                    println!("\nInvalid option! Press Enter to continue...");
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                }
            }
        }
    }

    fn perform_clean_exit(&self) {
        let context = "Menu::perform_clean_exit";
        log_info("Performing clean exit...", context);

        if self.click_service.is_enabled() {
            log_info("Disabling active click service before exit", context);
            self.click_service.toggle();

            thread::sleep(Duration::from_millis(100));
        }

        log_info("Clean exit completed, terminating process", context);

        std::process::exit(0);
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
        let settings = match Settings::load() {
            Ok(mut s) => {
                s.toggle_key = self.toggle_key;
                s
            },
            Err(_) => Settings::default_with_toggle_key(self.toggle_key),
        };
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
        let settings = match Settings::load() {
            Ok(mut s) => {
                s.toggle_key = self.toggle_key;
                s
            },
            Err(_) => Settings::default_with_toggle_key(self.toggle_key),
        };
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
        println!("Toggle Mode: {}", match self.toggle_mode {
            ToggleMode::MouseHold => "Mouse Hold",
            ToggleMode::KeyboardHold => "Keyboard Hold"
        });

        if let Ok(settings) = Settings::load() {
            println!("\n=== Advanced Settings ===");
            println!("Target Process: {}", settings.target_process);
            println!("Adaptive CPU Mode: {}", if settings.adaptive_cpu_mode { "Enabled" } else { "Disabled" });
            println!("Click Delay: {} microseconds", settings.click_delay_micros);
            println!("Delay Range: {}ms - {}ms", settings.delay_range_min, settings.delay_range_max);
            println!("Random Deviation: {} to {} microseconds", settings.random_deviation_min, settings.random_deviation_max);
            println!("Burst Mode: {}", if settings.burst_mode { "Enabled" } else { "Disabled" });
            println!("Game Mode (settings): {}", settings.game_mode);
        }

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
        let context = "Menu::start_auto_clicker";

        if self.toggle_key == 0 {
            self.clear_console();
            println!("Please configure hotkey first!");
            println!("\nPress Enter to continue...");

            let mut _input = String::new();
            if let Err(e) = io::stdin().read_line(&mut _input) {
                log_error(&format!("Failed to read continue prompt: {}", e), context);
            }
            return;
        }

        self.clear_console();

        match self.toggle_mode {
            ToggleMode::MouseHold => {
                println!("RAC Started! Press {} to enable/disable.", Self::get_key_name(self.toggle_key));
                println!("When enabled, hold LEFT MOUSE BUTTON to activate clicking.");
                println!("Note: If clicking stops, press the toggle key twice quickly to reset.");
            },
            ToggleMode::KeyboardHold => {
                println!("RAC Started!");
                println!("Hold {} to activate clicking.", Self::get_key_name(self.toggle_key));
                println!("Note: If clicking stops, press the toggle key twice quickly to reset.");
            }
        }

        self.run_main_loop();
    }

    fn run_main_loop(&self) {
        let mut last_toggle = Instant::now();
        let toggle_cooldown = Duration::from_millis(200);
        let loop_sleep_duration = Duration::from_millis(1);
        let mut system_enabled = false;
        let mut last_key_state = false;
        let mut key_hold_start: Option<Instant> = None;
        let key_hold_threshold = Duration::from_millis(10);

        match self.toggle_mode {
            ToggleMode::MouseHold => {
                println!("Press Ctrl + Q to return to main menu (only works when focused on this window)");
                println!("Mode: Mouse Hold - Press {} to enable, then hold LEFT MOUSE BUTTON to click",
                         Self::get_key_name(self.toggle_key));
            },
            ToggleMode::KeyboardHold => {
                println!("Press Ctrl + Q to return to main menu (only works when focused on this window)");
                println!("Mode: Keyboard Hold - Hold {} to click",
                         Self::get_key_name(self.toggle_key));
            }
        }

        loop {
            unsafe {
                static mut LAST_WINDOW_CHECK: Option<Instant> = None;
                let current_time = Instant::now();

                let mut is_our_window = false;
                if let Some(last_check) = LAST_WINDOW_CHECK {
                    if current_time.duration_since(last_check) >= Duration::from_millis(100) {
                        let foreground_window = GetForegroundWindow();
                        let mut title = [0u8; 256];
                        let len = GetWindowTextA(foreground_window, &mut title);
                        is_our_window = len > 0 &&
                            std::str::from_utf8_unchecked(&title[..len as usize]).contains("RAC");
                        LAST_WINDOW_CHECK = Some(current_time);
                    }
                } else {
                    LAST_WINDOW_CHECK = Some(current_time);
                }

                let ctrl_pressed = (GetAsyncKeyState(0x11) as u16 & 0x8000) != 0;
                let q_pressed = (GetAsyncKeyState(0x51) as u16 & 0x8000) != 0;
                if is_our_window && ctrl_pressed && q_pressed {
                    if self.click_service.is_enabled() {
                        self.click_service.force_disable_clicking();
                    }
                    break;
                }

                let key_pressed = (GetAsyncKeyState(self.toggle_key) as u16 & 0x8000) != 0;
                let toggle_pressed = key_pressed && !last_key_state;
                last_key_state = key_pressed;

                match self.toggle_mode {
                    ToggleMode::MouseHold => {
                        if toggle_pressed && current_time.duration_since(last_toggle) > toggle_cooldown {
                            system_enabled = !system_enabled;
                            last_toggle = current_time;

                            if self.click_service.is_enabled() {
                                self.click_service.force_disable_clicking();
                            }
                        }

                        if system_enabled {
                            let lmb_pressed = (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0;

                            if lmb_pressed && !self.click_service.is_enabled() {
                                self.click_service.force_enable_clicking();
                            } else if !lmb_pressed && self.click_service.is_enabled() {
                                self.click_service.force_disable_clicking();
                            }
                        } else if !system_enabled && self.click_service.is_enabled() {
                            self.click_service.force_disable_clicking();
                        }
                    },
                    ToggleMode::KeyboardHold => {
                        if key_pressed {
                            if key_hold_start.is_none() {
                                key_hold_start = Some(current_time);
                            }

                            let hold_duration = current_time.duration_since(key_hold_start.unwrap());
                            if hold_duration >= key_hold_threshold && !self.click_service.is_enabled() {
                                self.click_service.force_enable_clicking();
                            }
                        } else {
                            key_hold_start = None;
                            if self.click_service.is_enabled() {
                                self.click_service.force_disable_clicking();
                            }
                        }
                    }
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

            0x41..=0x5A => format!("Key {}", key as u8 as char),

            _ => format!("Unknown Key (0x{:02X})", key),
        }
    }
    
    fn configure_advanced_settings(&mut self) {
        let context = "Menu::configure_advanced_settings";
        let mut settings = match Settings::load() {
            Ok(s) => s,
            Err(e) => {
                log_error(&format!("Failed to load settings: {}", e), context);
                println!("\nFailed to load settings. Press Enter to continue...");
                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);
                return;
            }
        };

        loop {
            self.clear_console();
            println!("=== Advanced Settings Configuration ===");
            println!("1. Target Process: {}", settings.target_process);
            println!("2. Adaptive CPU Mode: {}", if settings.adaptive_cpu_mode { "Enabled" } else { "Disabled" });
            println!("3. Click Delay: {} microseconds", settings.click_delay_micros);
            println!("4. Delay Range: {}ms - {}ms", settings.delay_range_min, settings.delay_range_max);
            println!("5. Random Deviation: {} to {} microseconds", settings.random_deviation_min, settings.random_deviation_max);
            println!("6. Burst Mode: {}", if settings.burst_mode { "Enabled" } else { "Disabled" });
            println!("7. Save and Return to Main Menu");
            print!("\nSelect option to change: ");


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
                "1" => {
                    println!("Enter target process name (current: {}): ", settings.target_process);
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    let input = input.trim();
                    if !input.is_empty() {
                        settings.target_process = input.to_string();
                    }
                }
                "2" => {
                    println!("Toggle Adaptive CPU Mode (currently {})", if settings.adaptive_cpu_mode { "Enabled" } else { "Disabled" });
                    println!("1. Enable");
                    println!("2. Disable");
                    print!("Enter choice: ");
                    
                    if let Err(e) = io::stdout().flush() {
                        log_error(&format!("Failed to flush stdout: {}", e), context);
                        continue;
                    }
                    
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    match input.trim() {
                        "1" => settings.adaptive_cpu_mode = true,
                        "2" => settings.adaptive_cpu_mode = false,
                        _ => {
                            println!("Invalid choice. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                        }
                    }
                }
                "3" => {
                    println!("Enter click delay in microseconds (current: {}): ", settings.click_delay_micros);
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    if let Ok(value) = input.trim().parse::<u64>() {
                        if value > 0 {
                            settings.click_delay_micros = value;
                        } else {
                            println!("Value must be greater than 0. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                        }
                    } else {
                        println!("Invalid number. Press Enter to continue...");
                        let mut _input = String::new();
                        let _ = io::stdin().read_line(&mut _input);
                    }
                }
                "4" => {
                    println!("Enter delay range minimum in milliseconds (current: {}): ", settings.delay_range_min);
                    let mut min_input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut min_input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    let min_value = if let Ok(value) = min_input.trim().parse::<f64>() {
                        if value > 0.0 {
                            value
                        } else {
                            println!("Value must be greater than 0. Using current value.");
                            settings.delay_range_min
                        }
                    } else {
                        println!("Invalid number. Using current value.");
                        settings.delay_range_min
                    };
                    
                    println!("Enter delay range maximum in milliseconds (current: {}): ", settings.delay_range_max);
                    let mut max_input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut max_input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    let max_value = if let Ok(value) = max_input.trim().parse::<f64>() {
                        if value > min_value {
                            value
                        } else {
                            println!("Value must be greater than minimum. Using current value.");
                            settings.delay_range_max
                        }
                    } else {
                        println!("Invalid number. Using current value.");
                        settings.delay_range_max
                    };
                    
                    settings.delay_range_min = min_value;
                    settings.delay_range_max = max_value;
                }
                "5" => {
                    println!("Enter random deviation minimum in microseconds (current: {}): ", settings.random_deviation_min);
                    let mut min_input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut min_input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    let min_value = if let Ok(value) = min_input.trim().parse::<i32>() {
                        value
                    } else {
                        println!("Invalid number. Using current value.");
                        settings.random_deviation_min
                    };
                    
                    println!("Enter random deviation maximum in microseconds (current: {}): ", settings.random_deviation_max);
                    let mut max_input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut max_input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    let max_value = if let Ok(value) = max_input.trim().parse::<i32>() {
                        if value >= min_value {
                            value
                        } else {
                            println!("Value must be greater than or equal to minimum. Using current value.");
                            settings.random_deviation_max
                        }
                    } else {
                        println!("Invalid number. Using current value.");
                        settings.random_deviation_max
                    };
                    
                    settings.random_deviation_min = min_value;
                    settings.random_deviation_max = max_value;
                }
                "6" => {
                    println!("Toggle Burst Mode (currently {})", if settings.burst_mode { "Enabled" } else { "Disabled" });
                    println!("1. Enable");
                    println!("2. Disable");
                    print!("Enter choice: ");

                    if let Err(e) = io::stdout().flush() {
                        log_error(&format!("Failed to flush stdout: {}", e), context);
                        continue;
                    }

                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }

                    match input.trim() {
                        "1" => {
                            settings.burst_mode = true;
                            if let Ok(mut delay_provider) = self.click_service.delay_provider.lock() {
                                delay_provider.toggle_burst_mode();
                            }
                        },
                        "2" => {
                            settings.burst_mode = false;
                            if let Ok(mut delay_provider) = self.click_service.delay_provider.lock() {
                                if delay_provider.burst_mode {
                                    delay_provider.toggle_burst_mode();
                                }
                            }
                        },
                        _ => {
                            println!("Invalid choice. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                        }
                    }
                },
                "7" => {
                    if let Err(e) = settings.save() {
                        log_error(&format!("Failed to save settings: {}", e), context);
                        println!("\nFailed to save settings. Press Enter to continue...");
                        let mut _input = String::new();
                        let _ = io::stdin().read_line(&mut _input);
                    } else {
                        println!("\nSettings saved successfully! Press Enter to continue...");
                        let mut _input = String::new();
                        let _ = io::stdin().read_line(&mut _input);
                    }
                    return;
                }
                _ => {
                    println!("Invalid option. Press Enter to continue...");
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                }
            }
        }
    }
}