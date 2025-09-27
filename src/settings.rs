use crate::{FromBinary, ToBinary};
use std::io::Read;
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
                                "difficulty" => thing!(difficulty),
                                "fast_mode" => thing!(fast_mode),
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
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Difficulty {
    Normal,
    Easy,
}
impl Difficulty {
    pub fn is_easy(self) -> bool {
        matches!(self, Difficulty::Easy)
    }
}
impl std::str::FromStr for Difficulty {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "normal" => Self::Normal,
            "easy" => Self::Easy,
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
            }
        )
    }
}
