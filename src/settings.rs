use crate::board::AxisLength;
use crate::math::*;
use abes_nice_things::Style;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;
use std::fs::File;
use std::io::Read;
use std::io::Write;

macro_rules! settings {
    // The macro call which does
    ($setting_type:tt as $($type:ty, $field:ident, $name:literal, $default:expr, $values:expr);*) => {
        settings!($setting_type | $($type, $field, $name, $default, $values);*);
        impl ToBinary for $setting_type {
            fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
                $(self.$field.value.as_ref().to_binary(binary)?;)*
                Ok(())
            }
        }
        impl FromBinary for $setting_type {
            fn from_binary(binary: &mut dyn Read) -> Result<Self> {
                let mut out = Self::new();
                $(out.$field.value = <Option<$type>>::from_binary(binary)?;)*
                Ok(out)
            }
        }
    };
    // The macro call which does not implement binary conversions
    ($setting_type:tt | $($type:ty, $field:ident, $name:literal, $default:expr, $values:expr);*) => {
        /// Settings that can be changed at any time and do not get saved by [State]. Things like personal
        /// preferences rather than game affecting things
        #[derive(Debug)]
        pub struct $setting_type {
            $($field: Setting<$type>,)*
        }
        impl $setting_type {
            /// Creates an instance with no data only defaults
            fn new() -> Self {
                Self {
                    $($field: Setting::new($name, $default, $values),)*
                }
            }
            /// Returns if it set a value
            fn try_set(&mut self, field: &str, value: &str) -> bool {
                match field {
                    $(
                        $name => if let Ok(value) = value.parse::<$type>() {
                            self.$field.value = Some(value);
                            return true
                        },
                    )*
                    _ => {}
                }
                false
            }
            /// Saves the contents to a file but only saves non-default content
            fn save_to_file(&self, file: &mut File) {
                $(
                    if let Some(value) = self.$field.value {
                        writeln!(file, "{}={}", self.$field.name, value).unwrap();
                    }
                )*
            }
            fn get_all_names(&self) -> &'static[&'static str] {
                &[$(
                    $name,
                )*]
            }
            pub fn get_names_and_values(&self) -> Vec<(&'static str, String)> {
                vec![$(
                    ($name, format!("{}", self.$field.value.as_ref().unwrap_or(&self.$field.default))),
                )*]
            }
            /// Set the cursor to the start of the value and clear until end of line before running
            /// this, k?
            /// Returns if it found a field to set, not if it set it
            pub fn prompt_set_value(&mut self, name: &str) -> bool {
                match name {$(
                    $name => {
                        // Handle the easy one first
                        if self.$field.values.is_none() {
                            crate::input::normalize().unwrap();
                            if let Ok(new) = abes_nice_things::input().parse::<$type>() {
                                self.$field.value = Some(new);
                            }
                            crate::input::weirdify().unwrap();
                        }
                        // Now for the not so easy one
                        else {
                            let mut values = self.$field.values.as_ref().unwrap().iter();
                            while let Some(value) = values.next() && value != self.$field.value.as_ref().unwrap_or(&self.$field.default) {}
                            let new = values.next().unwrap_or(&self.$field.values.unwrap()[0]).clone();
                            self.$field.value = Some(new);
                        }
                    }
                )*
                    _ => return false,
                }
                true
            }
            $(
            /// Gets the value of the same name
            pub fn $field(&self) -> &$type {
                self.$field.value.as_ref().unwrap_or(&self.$field.default)
            }
            )*
        }
    }
}
// Unlocked settings
settings!(
    UnlockedSettings |
    bool,
    kick_doors,
    "kick doors",
    true,
    Some(&[true, false]);

    bool,
    kick_enemies,
    "kick enemies",
    true,
    Some(&[true, false])
);
// Locked settings
settings!(
    LockedSettings as AxisLength,
    axis_length,
    "map size",
    AxisLength::Full,
    Some(&[AxisLength::Small, AxisLength::Full])
);

/// Settings that can only be changed in between runs and will be saved and loaded with [State].
/// This is for things which affect game logic like difficulty

#[derive(Debug)]
struct Setting<T: std::fmt::Display + std::str::FromStr + PartialEq + Clone + 'static> {
    /// The name to be shown in the setting picker and used in the file, it MUST not contain any
    /// spaces
    name: &'static str,
    /// The default value if it cannot be loaded from the file
    default: T,
    /// The value to be used, if this is None then it will fall back to the given default
    value: Option<T>,
    /// The values to loop through when picking a value, if this is None then the user will have to
    /// type in the value instead
    values: Option<&'static [T]>,
}
impl<T: std::fmt::Display + std::str::FromStr + PartialEq + Clone + 'static> Setting<T> {
    fn new(name: &'static str, default: T, values: Option<&'static [T]>) -> Setting<T> {
        Setting {
            name,
            default,
            value: None,
            values,
        }
    }
}

pub fn load_from_file() -> (UnlockedSettings, LockedSettings) {
    let mut unlocked = UnlockedSettings::new();
    let mut locked = LockedSettings::new();

    // If there is no file then just use the defaults
    if let Ok(mut file) = File::open("settings") {
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        for line in data.lines() {
            let mut pieces = line.split('=');
            let field = pieces.next().unwrap().trim();
            let value = pieces.next().unwrap().trim();
            let _ = unlocked.try_set(field, value) || locked.try_set(field, value);
        }
    }

    (unlocked, locked)
}
pub fn load_unlocked_settings() -> UnlockedSettings {
    let mut unlocked = UnlockedSettings::new();
    if let Ok(mut file) = File::open("settings") {
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        for line in data.lines() {
            let mut pieces = line.split("=");
            let field = pieces.next().unwrap().trim();
            let value = pieces.next().unwrap().trim();
            unlocked.try_set(field, value);
        }
    }
    unlocked
}
pub fn save_to_file(unlocked: &UnlockedSettings, locked: &LockedSettings) {
    let mut file = File::create("settings").unwrap();
    unlocked.save_to_file(&mut file);
    locked.save_to_file(&mut file);
}

pub fn settings_editor() {
    let (mut unlocked, mut locked) = load_from_file();
    crate::input::weirdify().unwrap();
    let mut stdin = std::io::stdin();
    let mut buf = [0];
    let mut index = 0;
    // Whether or not we are currently selecting the locked list
    let mut select_locked = false;
    let unlocked_names = unlocked.get_all_names();
    let locked_names = locked.get_all_names();
    macro_rules! combined_index {
        () => {
            if select_locked {
                index + unlocked_names.len()
            } else {
                index
            }
        };
    }
    loop {
        println!("\x1b[H\x1b[0JQ to quit\n");
        let selected = *Style::new().background_red();
        for (name_index, (name, value)) in unlocked.get_names_and_values().iter().enumerate() {
            if !select_locked && name_index == index {
                print!("{selected}");
            }
            println!("{name}: {value}\x1b[0m");
        }
        for (name_index, (name, value)) in locked.get_names_and_values().iter().enumerate() {
            if select_locked && name_index == index {
                print!("{selected}");
            }
            println!("{name}: {value}\x1b[0m");
        }
        print!("\x1b[{};0H", combined_index!() + 3);
        std::io::stdout().flush().unwrap();

        stdin.read_exact(&mut buf).unwrap();
        let dir = match buf[0] {
            27 => {
                stdin.read_exact(&mut buf).unwrap();
                stdin.read_exact(&mut buf).unwrap();
                match buf[0] {
                    b'A' => Direction::Up,
                    b'B' => Direction::Down,
                    _ => continue,
                }
            }
            b'w' => Direction::Up,
            b's' => Direction::Down,
            b'\n' => {
                let combined = combined_index!();
                let shift = if select_locked {
                    locked_names[index].len()
                } else {
                    unlocked_names[index].len()
                } + 3;
                print!("\x1b[{};{}H\x1b[0K", combined + 3, shift);
                std::io::stdout().flush().unwrap();
                if select_locked {
                    locked.prompt_set_value(locked_names[index]);
                } else {
                    unlocked.prompt_set_value(unlocked_names[index]);
                }
                continue;
            }
            b'q' => {
                crate::input::normalize().unwrap();
                break;
            }
            _ => continue,
        };
        (index, select_locked) = match (dir, select_locked) {
            (Direction::Up, false) => {
                if index == 0 {
                    (locked_names.len() - 1, true)
                } else {
                    (index - 1, false)
                }
            }
            (Direction::Down, false) => {
                if index + 1 == unlocked_names.len() {
                    (0, true)
                } else {
                    (index + 1, false)
                }
            }
            (Direction::Up, true) => {
                if index == 0 {
                    (unlocked_names.len() - 1, false)
                } else {
                    (index - 1, true)
                }
            }
            (Direction::Down, true) => {
                if index + 1 == locked_names.len() {
                    (0, false)
                } else {
                    (index + 1, true)
                }
            }
            _ => continue,
        }
    }
    save_to_file(&unlocked, &locked);
}
