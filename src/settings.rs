use crate::{FromBinary, ToBinary};
use std::io::{Read, Write};
const PATH: &str = "settings";
#[derive(Debug, Clone)]
pub struct Settings {
    pub kick_enemies: bool,
    pub kick_doors: bool,
    pub difficulty: Difficulty,
    pub fast_mode: bool,
}
const DEFAULT_FILE: &[u8] = include_bytes!("default_settings.txt");
impl Settings {
    pub fn get_from_file() -> Settings {
        let difficulty_choice = std::cell::LazyCell::new(|| {
            crate::log!("Getting difficulty from player choice");
            // Need to get the player to choose a difficulty
            println!("Select your difficulty.\n1: easy\n2: normal\n3: hard");
            std::io::stdout().flush().unwrap();
            let mut stdin = std::io::stdin().lock();
            let mut buf = [0];
            let chosen = loop {
                stdin.read_exact(&mut buf).unwrap();
                match buf[0] {
                    b'1' => break Difficulty::Easy,
                    b'2' => break Difficulty::Normal,
                    b'3' => break Difficulty::Hard,
                    _ => {}
                }
            };
            std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(PATH)
                .unwrap()
                .write_all(format!("difficulty {chosen}").as_bytes())
                .unwrap();
            chosen
        });
        match std::fs::exists(PATH).unwrap() {
            true => {
                let mut contents = String::new();
                std::fs::File::open("settings")
                    .unwrap()
                    .read_to_string(&mut contents)
                    .unwrap();
                let mut settings = Settings::default();
                let mut difficulty_was_not_set = true;
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
                            macro_rules! thing {
                                ($field:ident) => {
                                    settings.$field = match value.parse() {
                                        Ok(field) => field,
                                        Err(_) => continue,
                                    }
                                };
                            }
                            match field {
                                "kick_enemies" => thing!(kick_enemies),
                                "kick_doors" => thing!(kick_doors),
                                "difficulty" => {
                                    difficulty_was_not_set = false;
                                    thing!(difficulty)
                                }
                                "fast_mode" => thing!(fast_mode),
                                _ => {}
                            }
                        }
                    }
                }
                if difficulty_was_not_set {
                    settings.difficulty = *difficulty_choice;
                }
                crate::log!("Settings chosen: {settings:?}");
                settings
            }
            false => {
                std::fs::write("settings", DEFAULT_FILE).unwrap();
                let mut settings = Settings::default();
                settings.difficulty = *difficulty_choice;
                settings
            }
        }
    }
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            kick_enemies: true,
            kick_doors: true,
            difficulty: Difficulty::default(),
            fast_mode: false,
        }
    }
}
impl FromBinary for Settings {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Settings {
            kick_enemies: bool::from_binary(binary)?,
            kick_doors: bool::from_binary(binary)?,
            difficulty: Difficulty::from_binary(binary)?,
            fast_mode: bool::from_binary(binary)?,
        })
    }
}
impl ToBinary for Settings {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        self.kick_enemies.to_binary(binary)?;
        self.kick_doors.to_binary(binary)?;
        self.difficulty.to_binary(binary)?;
        self.fast_mode.to_binary(binary)
    }
}
#[derive(PartialEq, Eq, Clone, Copy, Debug, Ord)]
pub enum Difficulty {
    Normal,
    Easy,
    Hard,
}
impl std::str::FromStr for Difficulty {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "normal" => Self::Normal,
            "easy" => Self::Easy,
            "hard" => Self::Hard,
            _ => return Err(()),
        })
    }
}
impl Default for Difficulty {
    fn default() -> Self {
        Self::Normal
    }
}
impl FromBinary for Difficulty {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Self::Normal,
            1 => Self::Easy,
            2 => Self::Hard,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to get Difficulty from binary",
                ));
            }
        })
    }
}
impl ToBinary for Difficulty {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        match self {
            Self::Normal => 0_u8,
            Self::Easy => 1,
            Self::Hard => 2,
        }
        .to_binary(binary)
    }
}
impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Normal => "normal",
                Self::Easy => "easy",
                Self::Hard => "hard",
            }
        )
    }
}
impl PartialOrd for Difficulty {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            return Some(std::cmp::Ordering::Equal);
        }
        Some(match self {
            Difficulty::Normal => match other {
                Difficulty::Easy => std::cmp::Ordering::Greater,
                Difficulty::Hard => std::cmp::Ordering::Less,
                _ => unreachable!(),
            },
            Difficulty::Easy => match other {
                Difficulty::Normal | Difficulty::Hard => std::cmp::Ordering::Less,
                _ => unreachable!(),
            },
            Difficulty::Hard => match other {
                Difficulty::Normal | Difficulty::Easy => std::cmp::Ordering::Greater,
                _ => unreachable!(),
            },
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn difficulty_partial_ord() {
        assert!(Difficulty::Easy < Difficulty::Normal);
        assert!(Difficulty::Normal < Difficulty::Hard);
        assert!(Difficulty::Easy < Difficulty::Hard);
    }
}
