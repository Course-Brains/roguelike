use std::io::Read;
pub struct Settings {
    pub kick_enemies: bool,
    pub kick_doors: bool,
}
const DEFAULT_FILE: &[u8] = include_bytes!("default_settings.txt");
macro_rules! err_continue {
    ($($token:tt)*) => {
        match $($token)* {
            Ok(val) => val,
            Err(_) => continue,
        }
    };
}
impl Settings {
    pub fn get_from_file() -> Settings {
        match std::fs::exists("settings").unwrap() {
            true => {
                let mut contents = String::new();
                std::fs::File::open("settings")
                    .unwrap()
                    .read_to_string(&mut contents)
                    .unwrap();
                let mut settings = Settings::default();
                for line in contents.lines() {
                    let line = line.trim();
                    if line.starts_with("//") {
                        continue;
                    }
                    if line.len() == 0 {
                        continue;
                    }
                    let mut iter = line.split(' ');
                    if let Some(field) = iter.next() {
                        if let Some(value) = iter.next() {
                            match field {
                                "kick_enemies" => {
                                    settings.kick_enemies = err_continue!(value.parse())
                                }
                                "kick_doors" => settings.kick_doors = err_continue!(value.parse()),
                                _ => {}
                            }
                        }
                    }
                }
                settings
            }
            false => {
                std::fs::write("settings", DEFAULT_FILE).unwrap();
                Settings::default()
            }
        }
    }
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            kick_enemies: true,
            kick_doors: true,
        }
    }
}
