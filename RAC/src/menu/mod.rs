use crate::config::settings::Settings;
use crate::input::click_service::ClickService;
use crate::input::click_executor::{ClickExecutor, GameMode, MouseButton};
use crate::logger::logger::{log_error, log_info};
use std::io::{self, Write};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use windows::core::PCSTR;
use windows::Win32::System::Console::SetConsoleTitleA;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode, Clear, ClearType};
use crossterm::execute;

#[derive(Clone, Copy, PartialEq)]
enum ToggleMode {
    MouseHold,
    KeyboardHold,
}

#[derive(Clone, Copy, PartialEq)]
enum ClickMode {
    LeftClick,
    RightClick,
    Both
}

pub struct Menu {
    click_service: Arc<ClickService>,
    click_executor: Arc<ClickExecutor>,
    toggle_key: i32,
    toggle_mode: ToggleMode,
    click_mode: ClickMode,
    settings: Settings,
}

impl Menu {
    pub fn new(click_service: Arc<ClickService>, click_executor: Arc<ClickExecutor>) -> Self {
        let context = "Menu::new";

        let settings = match Settings::load() {
            Ok(s) => {
                log_info("Loaded existing configuration", context);

                let left_executor = click_service.get_left_click_executor();
                let left_mode = match s.left_game_mode.as_str() {
                    "Combo" => GameMode::Combo,
                    _ => GameMode::Default, 
                };
                left_executor.set_game_mode(left_mode);
                
                let right_executor = click_service.get_right_click_executor();
                let right_mode = match s.right_game_mode.as_str() {
                    "Combo" => GameMode::Combo,
                    _ => GameMode::Default,
                };
                right_executor.set_game_mode(right_mode);

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
            click_mode: ClickMode::LeftClick,
            settings,
        };

        menu.start_toggle_monitor();

        log_info("Menu initialized successfully", context);
        menu
    }

    fn clear_console(&self) {
        if let Err(_) = execute!(io::stdout(), Clear(ClearType::All)) {
            print!("\x1B[2J\x1B[3J\x1B[1;1H");
        }
        
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("cmd").args(["/c", "cls"]).status();
        }

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
                let settings = match Settings::load() {
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
                let settings = match Settings::load() {
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

    fn configure_click_mode(&mut self) {
        let context = "Menu::configure_click_mode";

        self.clear_console();
        println!("=== Click Mode Configuration ===");
        println!("1. Left Click Mode");
        println!("2. Right Click Mode");
        println!("3. Both (Left and Right)");
        println!("4. Back to Main Menu");
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
                self.click_mode = ClickMode::LeftClick;
                self.click_executor.set_mouse_button(MouseButton::Left);

                let mut settings = match Settings::load() {
                    Ok(s) => s,
                    Err(_) => Settings::default(),
                };

                settings.click_mode = "LeftClick".to_string();

                if let Err(e) = settings.save() {
                    log_error(&format!("Failed to save settings: {}", e), context);
                    println!("Failed to save settings! Press Enter to continue...");
                } else {
                    println!("Left Click Mode enabled! Press Enter to continue...");
                }

                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);
            },
            "2" => {
                self.click_mode = ClickMode::RightClick;
                self.click_executor.set_mouse_button(MouseButton::Right);

                let mut settings = match Settings::load() {
                    Ok(s) => s,
                    Err(_) => Settings::default(),
                };

                settings.click_mode = "RightClick".to_string();

                if let Err(e) = settings.save() {
                    log_error(&format!("Failed to save settings: {}", e), context);
                    println!("Failed to save settings! Press Enter to continue...");
                } else {
                    println!("Right Click Mode enabled! Press Enter to continue...");
                }

                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);
            },
            "3" => {
                self.click_mode = ClickMode::Both;
                self.click_executor.set_mouse_button(MouseButton::Left);
                self.click_executor.set_mouse_button(MouseButton::Right);

                let mut settings = match Settings::load() {
                    Ok(s) => s,
                    Err(_) => Settings::default(),
                };

                settings.click_mode = "Both".to_string();

                if let Err(e) = settings.save() {
                    log_error(&format!("Failed to save settings: {}", e), context);
                    println!("Failed to save settings! Press Enter to continue...");
                } else {
                    println!("Both Click Mode enabled! Press Enter to continue...");
                }

                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);
            },
            "4" => return,
            _ => {
                log_error("Invalid click mode option selected", context);
                println!("\nInvalid option! Press Enter to continue...");
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
            println!("6. Configure Click Mode");
            println!("7. Exit");
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
                },
                "4" => self.configure_advanced_settings(),
                "5" => self.configure_toggle_mode(),
                "6" => self.configure_click_mode(),
                "7" => self.perform_clean_exit(),
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

        if let Err(e) = enable_raw_mode() {
            log_error(&format!("Failed to enable raw mode: {}", e), context);
            return;
        }

        let start_time = Instant::now();
        let timeout = Duration::from_secs(30);
        let mut input_received = false;

        while start_time.elapsed() < timeout && !input_received {
            if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(Event::Key(KeyEvent { code, .. })) = event::read() {
                    if let KeyCode::Char(c) = code {
                        if c.is_ascii_alphabetic() {
                            let virtual_key = c.to_ascii_uppercase() as i32;

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
                            }
                            input_received = true;
                        } else {
                            println!("\nInvalid key! Please press a letter key (A-Z)...");
                            thread::sleep(Duration::from_secs(2));
                            disable_raw_mode().unwrap_or(());
                            return;
                        }
                    }
                }
            }
        }

        let _ = disable_raw_mode();

        if !input_received {
            println!("\nTimeout reached! No key was pressed within {} seconds.", timeout.as_secs());
        }

        println!("Press Enter to continue...");
        let mut _input = String::new();
        let _ = io::stdin().read_line(&mut _input);
    }

    fn configure_mouse_hotkey(&mut self) {
        let context = "Menu::configure_mouse_hotkey";
        self.clear_console();
        println!("=== Mouse Hotkey Configuration ===");
        println!("\nPress any mouse button to set as hotkey...");

        if let Err(e) = io::stdout().flush() {
            log_error(&format!("Failed to flush stdout: {}", e), context);
            return;
        }

        let mut mouse_key = 0;
        let button_codes = [
            0x01, 0x02, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
            0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7,
            0xA8, 0xA9, 0xAA, 0xAB,
            0xAD, 0xAE, 0xAF, 0xB0, 0xB1, 0xB2, 0xB3
        ];

        let start_time = Instant::now();
        let timeout = Duration::from_secs(30);

        'detection: while mouse_key == 0 && start_time.elapsed() < timeout {
            for &key in &button_codes {
                unsafe {
                    let state = GetAsyncKeyState(key);
                    if (state as u16 & 0x8000) != 0 {
                        mouse_key = key;
                        thread::sleep(Duration::from_millis(100));
                        break 'detection;
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        if mouse_key == 0 {
            println!("\nTimeout reached! No button was pressed within {} seconds.", timeout.as_secs());
            println!("\nPress Enter to continue...");
            let mut _input = String::new();
            let _ = io::stdin().read_line(&mut _input);
            return;
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
            println!("\nHotkey successfully set to: {} (code: 0x{:02X})",
                     Self::get_key_name(mouse_key), mouse_key);
            println!("To change the hotkey, return to the main menu and configure again.");
            println!("\nPress Enter to continue...");

            let mut _input = String::new();
            if let Err(e) = io::stdin().read_line(&mut _input) {
                log_error(&format!("Failed to read continue prompt: {}", e), context);
            }
        }
    }

    fn show_current_settings(&self) {
        let context = "Menu::show_current_settings";
        
        let settings = match Settings::load() {
            Ok(s) => s,
            Err(_) => {
                log_error("Failed to load settings", context);
                println!("Failed to load settings. Press Enter to continue...");
                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);
                return;
            }
        };
        
        self.clear_console();
        println!("=== Current Settings ===\n");
        
        println!("Toggle Key: {}", Self::get_key_name(settings.toggle_key));
        println!("Toggle Mode: {}", if settings.keyboard_hold_mode { "Keyboard Hold" } else { "Mouse Hold" });
        println!("Target Process: {}", settings.target_process);
        println!("Adaptive CPU Mode: {}", if settings.adaptive_cpu_mode { "Enabled" } else { "Disabled" });
        
        println!("\n=== Left Click Settings ===");
        println!("1. Max CPS: {} (Clicks Per Second)", settings.left_max_cps);
        println!("2. Randomize Click Delay: {}", if settings.left_game_mode == "Combo" { "Enabled" } else { "Disabled" });
        println!("3. Click Delay: {} microseconds", settings.left_click_delay_micros);
        println!("4. Random Deviation: {} to {} microseconds", settings.left_random_deviation_min, settings.left_random_deviation_max);
        
        println!("\n=== Right Click Settings ===");
        println!("Max CPS: {}", settings.right_max_cps);
        println!("Executor CPS: {}", self.click_service.get_right_click_executor().get_current_max_cps());
        println!("Randomize Click Delay: {}", if settings.right_game_mode == "Combo" { "Enabled" } else { "Disabled" });
        println!("Click Delay: {} microseconds", settings.right_click_delay_micros);
        println!("Random Deviation: {} to {} microseconds", settings.right_random_deviation_min, settings.right_random_deviation_max);
        
        println!("\nPress Enter to continue...");
        let mut _input = String::new();
        let _ = io::stdin().read_line(&mut _input);
    }

    fn start_auto_clicker(&mut self) {
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

        let settings = Settings::load().unwrap_or_default();

        self.click_mode = match settings.click_mode.as_str() {
            "LeftClick" => ClickMode::LeftClick,
            "RightClick" => ClickMode::RightClick,
            "Both" => ClickMode::Both,
            _ => ClickMode::LeftClick,
        };

        self.apply_settings();

        match self.click_mode {
            ClickMode::LeftClick => {
                self.click_service.force_enable_left_clicking();
                self.click_service.force_disable_right_clicking();
                let left_executor = self.click_service.get_left_click_executor();
                left_executor.set_mouse_button(MouseButton::Left);
                left_executor.set_max_cps(settings.left_max_cps);
                left_executor.set_active(true);
                let mode = match self.settings.left_game_mode.as_str() {
                    "Combo" => GameMode::Combo,
                    _ => GameMode::Default,
                };
                left_executor.set_game_mode(mode);
            },
            ClickMode::RightClick => {
                self.click_service.force_enable_right_clicking();
                self.click_service.force_disable_left_clicking();
                let right_executor = self.click_service.get_right_click_executor();
                right_executor.set_mouse_button(MouseButton::Right);
                right_executor.set_max_cps(settings.right_max_cps);
                right_executor.set_active(true);
                let mode = match self.settings.right_game_mode.as_str() {
                    "Combo" => GameMode::Combo,
                    _ => GameMode::Default,
                };
                right_executor.set_game_mode(mode);
                log_info("Right click mode activated", context);
            },
            ClickMode::Both => {
                self.click_service.force_enable_left_clicking();
                self.click_service.force_enable_right_clicking();
                let left_executor = self.click_service.get_left_click_executor();
                left_executor.set_mouse_button(MouseButton::Left);
                left_executor.set_max_cps(settings.left_max_cps);
                left_executor.set_active(true);
                let left_mode = match self.settings.left_game_mode.as_str() {
                    "Combo" => GameMode::Combo,
                    _ => GameMode::Default,
                };
                left_executor.set_game_mode(left_mode);

                let right_executor = self.click_service.get_right_click_executor();
                right_executor.set_mouse_button(MouseButton::Right);
                right_executor.set_max_cps(settings.right_max_cps);
                right_executor.set_active(true);
                let right_mode = match self.settings.right_game_mode.as_str() {
                    "Combo" => GameMode::Combo,
                    _ => GameMode::Default,
                };
                right_executor.set_game_mode(right_mode);
            }
        }

        match self.toggle_mode {
            ToggleMode::MouseHold => {
                println!("RAC Started! Press {} to enable/disable.", Self::get_key_name(self.toggle_key));
                println!("When enabled, hold mouse button to activate clicking.");
                match self.click_mode {
                    ClickMode::LeftClick => println!("Click Mode: LEFT CLICK"),
                    ClickMode::RightClick => println!("Click Mode: RIGHT CLICK"),
                    ClickMode::Both => println!("Click Mode: BOTH BUTTONS"),
                }
                println!("Press Ctrl+Q to return to menu.");
                println!("Note: If clicking stops, press the toggle key twice quickly to reset.");
            },
            ToggleMode::KeyboardHold => {
                println!("RAC Started!");
                println!("Hold {} to activate clicking.", Self::get_key_name(self.toggle_key));
                match self.click_mode {
                    ClickMode::LeftClick => println!("Click Mode: LEFT CLICK"),
                    ClickMode::RightClick => println!("Click Mode: RIGHT CLICK"),
                    ClickMode::Both => println!("Click Mode: BOTH BUTTONS"),
                }
                println!("Press Ctrl+Q to return to menu.");
                println!("Note: If clicking stops, press the toggle key twice quickly to reset.");
            }
        }

        self.run_main_loop();
    }

    fn run_main_loop(&self) {
        let context = "Menu::run_main_loop";

        if let Err(e) = enable_raw_mode() {
            log_error(&format!("Failed to enable raw mode: {}", e), context);
        }

        let quit_requested = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let quit_requested_clone = Arc::clone(&quit_requested);
        
        let key_thread = thread::spawn(move || {
            while !quit_requested_clone.load(std::sync::atomic::Ordering::Relaxed) {
                if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                    if let Ok(Event::Key(KeyEvent { code: KeyCode::Char('q'), modifiers, .. })) = event::read() {
                        if modifiers == event::KeyModifiers::CONTROL {
                            quit_requested_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                            break;
                        }
                    }
                }
            }
        });

        while !quit_requested.load(std::sync::atomic::Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
        }

        log_info("Ctrl+Q pressed, stopping RAC", context);
        
        self.click_service.force_disable_clicking();
        self.click_service.force_disable_left_clicking();
        self.click_service.force_disable_right_clicking();
        
        if let Err(e) = key_thread.join() {
            log_error(&format!("Failed to join key thread: {:?}", e), context);
        }
        
        if let Err(e) = disable_raw_mode() {
            log_error(&format!("Failed to disable raw mode: {}", e), context);
        }
    }

    fn configure_advanced_settings(&mut self) {
        let context = "Menu::configure_advanced_settings";
        let mut settings = match Settings::load() {
            Ok(s) => s,
            Err(_) => Settings::default(),
        };

        loop {
            self.clear_console();
            println!("=== Advanced Settings ===");
            println!("1. Configure Target Process (currently: {})", settings.target_process);
            println!("2. Toggle Adaptive CPU Mode (currently: {})", if settings.adaptive_cpu_mode { "Enabled" } else { "Disabled" });
            println!("3. Left Click Advanced Settings");
            println!("4. Right Click Advanced Settings");
            println!("5. Save and Return to Main Menu");
            print!("\nSelect option: ");

            if let Err(e) = io::stdout().flush() {
                log_error(&format!("Failed to flush stdout: {}", e), context);
                continue;
            }

            let mut choice = String::new();
            if let Err(e) = io::stdin().read_line(&mut choice) {
                log_error(&format!("Failed to read user input: {}", e), context);
                continue;
            }

            match choice.trim() {
                "1" => {
                    println!("Enter target process name (current: {}): ", self.settings.target_process);
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    let input = input.trim();
                    if !input.is_empty() {
                        self.settings.target_process = input.to_string();
                    }
                },
                "2" => {
                    println!("Toggle Adaptive CPU Mode (currently {})", if self.settings.adaptive_cpu_mode { "Enabled" } else { "Disabled" });
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
                        "1" => self.settings.adaptive_cpu_mode = true,
                        "2" => self.settings.adaptive_cpu_mode = false,
                        _ => {
                            println!("Invalid choice. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                            self.clear_console();
                        }
                    }
                },
                "3" => {
                    self.configure_left_click_settings();
                },
                "4" => {
                    self.configure_right_click_settings();
                },
                "5" => {
                    println!("Saving all settings...");
                    
                    let left_executor = self.click_service.get_left_click_executor();
                    left_executor.set_max_cps(self.settings.left_max_cps);
                    let left_mode = if self.settings.left_game_mode == "Combo" { GameMode::Combo } else { GameMode::Default };
                    left_executor.set_game_mode(left_mode);
                    
                    let right_executor = self.click_service.get_right_click_executor();
                    right_executor.force_right_cps(self.settings.right_max_cps);
                    
                    if let Err(e) = self.settings.save() {
                        log_error(&format!("Failed to save settings: {}", e), context);
                        println!("Failed to save settings! Press Enter to continue...");
                    } else {
                        println!("All settings saved successfully! Press Enter to continue...");
                    }
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                    return;
                },
                _ => {
                    println!("Invalid option. Press Enter to continue...");
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                }
            }
        }
    }

    fn configure_left_click_settings(&mut self) {
        let context = "Menu::configure_left_click_settings";
        
        loop {
            self.clear_console();
            println!("=== Left Click Settings ===");
            println!("1. Max CPS: {} (Clicks Per Second)", self.settings.left_max_cps);
            println!("2. Randomize Click Delay: {}", if self.settings.left_game_mode == "Combo" { "Enabled" } else { "Disabled" });
            println!("3. Click Delay Options");
            println!("4. Back to Advanced Settings");

            if let Err(e) = io::stdout().flush() {
                log_error(&format!("Failed to flush stdout: {}", e), context);
                return;
            }

            let mut choice = String::new();
            if let Err(e) = io::stdin().read_line(&mut choice) {
                log_error(&format!("Failed to read input: {}", e), context);
                return;
            }

            match choice.trim() {
                "1" => {
                    println!("Enter Left Max CPS (1-20) (current: {}): ", self.settings.left_max_cps);
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    if let Ok(value) = input.trim().parse::<u8>() {
                        if value > 0 {
                            self.settings.left_max_cps = value;
                            let left_executor = self.click_service.get_left_click_executor();
                            left_executor.set_max_cps(value);
                            
                            if let Err(e) = self.settings.save() {
                                log_error(&format!("Failed to save settings: {}", e), context);
                            } else {
                                log_info(&format!("Left click max CPS saved as {}", value), context);
                            }
                        }
                    }
                },
                "2" => {
                    self.clear_console();
                    println!("=== Randomize Click Delay ===");
                    println!("Current Status: {}", if self.settings.left_game_mode == "Combo" { "Enabled" } else { "Disabled" });
                    println!("\nOptions:");
                    println!("1. Disable (Uses constant speed based on Max CPS)");
                    println!("2. Enable (Adds random variations for natural clicking)");
                    
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    match input.trim() {
                        "1" => {
                            self.settings.left_game_mode = "Default".to_string();
                            let left_executor = self.click_service.get_left_click_executor();
                            left_executor.set_game_mode(GameMode::Default);
                            if let Err(e) = self.settings.save() {
                                log_error(&format!("Failed to save settings: {}", e), context);
                            }
                            println!("Randomize Click Delay disabled. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                        },
                        "2" => {
                            self.settings.left_game_mode = "Combo".to_string();
                            let left_executor = self.click_service.get_left_click_executor();
                            left_executor.set_game_mode(GameMode::Combo);
                            if let Err(e) = self.settings.save() {
                                log_error(&format!("Failed to save settings: {}", e), context);
                            }
                            println!("Randomize Click Delay enabled. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                        },
                        _ => {
                            println!("Invalid choice. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                        }
                    }
                },
                "3" => {
                    self.configure_left_click_delay_options();
                },
                "4" => return,
                _ => {
                    println!("Invalid option. Press Enter to continue...");
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                    self.clear_console();
                }
            }
        }
    }

    fn configure_left_click_delay_options(&mut self) {
        let context = "Menu::configure_left_click_delay_options";
        
        loop {
            self.clear_console();
            println!("=== Left Click Delay Options ===");
            println!("1. Click Delay: {} microseconds", self.settings.left_click_delay_micros);
            println!("2. Random Deviation: {} to {} microseconds", self.settings.left_random_deviation_min, self.settings.left_random_deviation_max);
            println!("3. Back to Left Click Settings");
            print!("\nSelect option: ");

            if let Err(e) = io::stdout().flush() {
                log_error(&format!("Failed to flush stdout: {}", e), context);
                return;
            }

            let mut choice = String::new();
            if let Err(e) = io::stdin().read_line(&mut choice) {
                log_error(&format!("Failed to read input: {}", e), context);
                return;
            }

            match choice.trim() {
                "1" => {
                    println!("Enter click delay in microseconds (current: {}): ", self.settings.left_click_delay_micros);
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    if let Ok(value) = input.trim().parse::<u64>() {
                        if value > 0 {
                            self.settings.left_click_delay_micros = value;
                        } else {
                            println!("Value must be greater than 0. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                            self.clear_console();
                        }
                    } else {
                        println!("Invalid number. Press Enter to continue...");
                        let mut _input = String::new();
                        let _ = io::stdin().read_line(&mut _input);
                        self.clear_console();
                    }
                },
                "2" => {
                    println!("Enter random deviation minimum in microseconds (current: {}): ", self.settings.left_random_deviation_min);
                    let mut min_input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut min_input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    let min_value = if let Ok(value) = min_input.trim().parse::<i32>() {
                        value
                    } else {
                        println!("Invalid number. Using current value.");
                        self.settings.left_random_deviation_min
                    };
                    
                    println!("Enter random deviation maximum in microseconds (current: {}): ", self.settings.left_random_deviation_max);
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
                            self.settings.left_random_deviation_max
                        }
                    } else {
                        println!("Invalid number. Using current value.");
                        self.settings.left_random_deviation_max
                    };
                    
                    self.settings.left_random_deviation_min = min_value;
                    self.settings.left_random_deviation_max = max_value;
                    self.clear_console();
                },
                "3" => return,
                _ => {
                    println!("Invalid option. Press Enter to continue...");
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                    self.clear_console();
                }
            }
        }
    }

    fn configure_right_click_settings(&mut self) {
        let context = "Menu::configure_right_click_settings";
        
        loop {
            self.clear_console();
            println!("=== Right Click Settings ===");
            println!("1. Max CPS: {} (Clicks Per Second)", self.settings.right_max_cps);
            println!("2. Randomize Click Delay: {}", if self.settings.right_game_mode == "Combo" { "Enabled" } else { "Disabled" });
            println!("3. Click Delay Options");
            println!("4. Back to Advanced Settings");

            if let Err(e) = io::stdout().flush() {
                log_error(&format!("Failed to flush stdout: {}", e), context);
                return;
            }

            let mut choice = String::new();
            if let Err(e) = io::stdin().read_line(&mut choice) {
                log_error(&format!("Failed to read input: {}", e), context);
                return;
            }

            match choice.trim() {
                "1" => {
                    println!("Enter new Max CPS (Clicks Per Second): ");
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    if let Ok(value) = input.trim().parse::<u8>() {
                        if value > 0 {
                            self.settings.right_max_cps = value;
                            
                            let right_executor = self.click_service.get_right_click_executor();
                            right_executor.set_max_cps(value);
                            
                            if let Err(e) = self.settings.save() {
                                log_error(&format!("Failed to save settings: {}", e), context);
                            }
                        }
                    }
                },
                "2" => {
                    self.clear_console();
                    println!("=== Randomize Click Delay ===");
                    println!("Current Status: {}", if self.settings.right_game_mode == "Combo" { "Enabled" } else { "Disabled" });
                    println!("\nOptions:");
                    println!("1. Disable (Uses constant speed based on Max CPS)");
                    println!("2. Enable (Adds random variations for natural clicking)");
                    
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    match input.trim() {
                        "1" => {
                            self.settings.right_game_mode = "Default".to_string();
                            let right_executor = self.click_service.get_right_click_executor();
                            right_executor.set_game_mode(GameMode::Default);
                            if let Err(e) = self.settings.save() {
                                log_error(&format!("Failed to save settings: {}", e), context);
                            }
                            println!("Randomize Click Delay disabled. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                        },
                        "2" => {
                            self.settings.right_game_mode = "Combo".to_string();
                            let right_executor = self.click_service.get_right_click_executor();
                            right_executor.set_game_mode(GameMode::Combo);
                            if let Err(e) = self.settings.save() {
                                log_error(&format!("Failed to save settings: {}", e), context);
                            }
                            println!("Randomize Click Delay enabled. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                        },
                        _ => {
                            println!("Invalid choice. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                        }
                    }
                },
                "3" => {
                    self.configure_right_click_delay_options();
                },
                "4" => return,
                _ => {
                    println!("Invalid option. Press Enter to continue...");
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                    self.clear_console();
                }
            }
        }
    }

    fn configure_right_click_delay_options(&mut self) {
        let context = "Menu::configure_right_click_delay_options";
        
        loop {
            self.clear_console();
            println!("=== Right Click Delay Options ===");
            println!("1. Click Delay: {} microseconds", self.settings.right_click_delay_micros);
            println!("2. Random Deviation: {} to {} microseconds", self.settings.right_random_deviation_min, self.settings.right_random_deviation_max);
            println!("3. Back to Right Click Settings");
            print!("\nSelect option: ");

            if let Err(e) = io::stdout().flush() {
                log_error(&format!("Failed to flush stdout: {}", e), context);
                return;
            }

            let mut choice = String::new();
            if let Err(e) = io::stdin().read_line(&mut choice) {
                log_error(&format!("Failed to read input: {}", e), context);
                return;
            }

            match choice.trim() {
                "1" => {
                    println!("Enter click delay in microseconds (current: {}): ", self.settings.right_click_delay_micros);
                    let mut input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    if let Ok(value) = input.trim().parse::<u64>() {
                        if value > 0 {
                            self.settings.right_click_delay_micros = value;
                        } else {
                            println!("Value must be greater than 0. Press Enter to continue...");
                            let mut _input = String::new();
                            let _ = io::stdin().read_line(&mut _input);
                            self.clear_console();
                        }
                    } else {
                        println!("Invalid number. Press Enter to continue...");
                        let mut _input = String::new();
                        let _ = io::stdin().read_line(&mut _input);
                        self.clear_console();
                    }
                },
                "2" => {
                    println!("Enter random deviation minimum in microseconds (current: {}): ", self.settings.right_random_deviation_min);
                    let mut min_input = String::new();
                    if let Err(e) = io::stdin().read_line(&mut min_input) {
                        log_error(&format!("Failed to read input: {}", e), context);
                        continue;
                    }
                    
                    let min_value = if let Ok(value) = min_input.trim().parse::<i32>() {
                        value
                    } else {
                        println!("Invalid number. Using current value.");
                        self.settings.right_random_deviation_min
                    };
                    
                    println!("Enter random deviation maximum in microseconds (current: {}): ", self.settings.right_random_deviation_max);
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
                            self.settings.right_random_deviation_max
                        }
                    } else {
                        println!("Invalid number. Using current value.");
                        self.settings.right_random_deviation_max
                    };
                    
                    self.settings.right_random_deviation_min = min_value;
                    self.settings.right_random_deviation_max = max_value;
                    self.clear_console();
                },
                "3" => return,
                _ => {
                    println!("Invalid option. Press Enter to continue...");
                    let mut _input = String::new();
                    let _ = io::stdin().read_line(&mut _input);
                    self.clear_console();
                }
            }
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

            0xA0..=0xB3 => format!("Special Button (0x{:02X})", key),
            0x41..=0x5A => format!("Key {}", key as u8 as char),
            _ => format!("Button Code 0x{:02X}", key),
        }
    }

    fn apply_settings(&mut self) {
        let mut settings = match Settings::load() {
            Ok(s) => s,
            Err(_) => Settings::default(),
        };
        
        if let Some(left_executor) = Arc::get_mut(&mut self.click_executor) {
            left_executor.set_max_cps(settings.left_max_cps);
            
            let mode = match settings.left_game_mode.as_str() {
                "Combo" => GameMode::Combo,
                _ => GameMode::Default,
            };
            left_executor.set_game_mode(mode);
            
            settings.left_game_mode = settings.left_game_mode.clone();
        }
        
        if let Ok(mut delay_provider) = self.click_service.delay_provider.lock() {
            if delay_provider.burst_mode != settings.burst_mode {
                delay_provider.toggle_burst_mode();
            }
        }
        
        if let Err(e) = settings.save() {
            log_error(&format!("Failed to save settings: {}", e), "Menu::apply_settings");
        }
    }

    fn toggle_service(&self) {
        static mut IS_ACTIVE: bool = false;
        
        unsafe {
            IS_ACTIVE = !IS_ACTIVE;
            
            if IS_ACTIVE {
                log_info("AutoClicker Enabled", "Menu::toggle_service");
                self.click_executor.set_active(true);
                
                if self.click_mode == ClickMode::Both || self.click_mode == ClickMode::RightClick {
                    self.click_service.get_right_click_executor().set_active(true);
                }
                
                if self.click_mode == ClickMode::Both || self.click_mode == ClickMode::LeftClick {
                    self.click_service.get_left_click_executor().set_active(true);
                }
            } else {
                log_info("AutoClicker Disabled", "Menu::toggle_service");
                self.click_executor.set_active(false);
                self.click_service.get_left_click_executor().set_active(false);
                self.click_service.get_right_click_executor().set_active(false);
            }
        }
    }

    fn start_toggle_monitor(&self) {
        let toggle_key = self.toggle_key;
        let left_executor = Arc::clone(&self.click_service.get_left_click_executor());
        let right_executor = Arc::clone(&self.click_service.get_right_click_executor());

        thread::spawn(move || {
            let mut was_pressed = false;
            let mut is_active = false;

            loop {
                let settings = Settings::load().unwrap_or_default();
                let click_mode = match settings.click_mode.as_str() {
                    "LeftClick" => ClickMode::LeftClick,
                    "RightClick" => ClickMode::RightClick,
                    "Both" => ClickMode::Both,
                    _ => ClickMode::LeftClick,
                };

                let toggle_mode = if settings.keyboard_hold_mode {
                    ToggleMode::KeyboardHold
                } else {
                    ToggleMode::MouseHold
                };

                let is_pressed = unsafe { (GetAsyncKeyState(toggle_key) & 0x8000u16 as i16) != 0 };

                match toggle_mode {
                    ToggleMode::MouseHold => {
                        if is_pressed && !was_pressed {
                            is_active = !is_active;

                            match click_mode {
                                ClickMode::LeftClick => {
                                    if is_active {
                                        left_executor.set_active(true);
                                        left_executor.set_mouse_button(MouseButton::Left);
                                        right_executor.set_active(false);
                                    } else {
                                        left_executor.set_active(false);
                                        right_executor.set_active(false);
                                    }
                                },
                                ClickMode::RightClick => {
                                    if is_active {
                                        right_executor.set_active(true);
                                        right_executor.set_mouse_button(MouseButton::Right);
                                        left_executor.set_active(false);
                                    } else {
                                        left_executor.set_active(false);
                                        right_executor.set_active(false);
                                    }
                                },
                                ClickMode::Both => {
                                    if is_active {
                                        left_executor.set_active(true);
                                        left_executor.set_mouse_button(MouseButton::Left);
                                        right_executor.set_active(true);
                                        right_executor.set_mouse_button(MouseButton::Right);
                                    } else {
                                        left_executor.set_active(false);
                                        right_executor.set_active(false);
                                    }
                                }
                            }
                        }
                    },
                    ToggleMode::KeyboardHold => {
                        if is_pressed != is_active {
                            is_active = is_pressed;

                            match click_mode {
                                ClickMode::LeftClick => {
                                    if is_active {
                                        left_executor.set_active(true);
                                        left_executor.set_mouse_button(MouseButton::Left);
                                        right_executor.set_active(false);
                                    } else {
                                        left_executor.set_active(false);
                                        right_executor.set_active(false);
                                    }
                                },
                                ClickMode::RightClick => {
                                    if is_active {

                                        right_executor.set_active(true);
                                        right_executor.set_mouse_button(MouseButton::Right);
                                        left_executor.set_active(false);
                                    } else {
                                        left_executor.set_active(false);
                                        right_executor.set_active(false);
                                    }
                                },
                                ClickMode::Both => {
                                    if is_active {
                                        left_executor.set_active(true);
                                        left_executor.set_mouse_button(MouseButton::Left);
                                        right_executor.set_active(true);
                                        right_executor.set_mouse_button(MouseButton::Right);
                                    } else {
                                        left_executor.set_active(false);
                                        right_executor.set_active(false);
                                    }
                                }
                            }
                        }
                    }
                }

                was_pressed = is_pressed;
                thread::sleep(Duration::from_millis(10));
            }
        });
    }
}