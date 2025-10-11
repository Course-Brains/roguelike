use crate::{FromBinary, ToBinary};
use std::fmt::Display;
use std::io::{Read, Write};
const PATH: &str = "settings";
#[derive(Debug, Clone)]
pub struct Settings {
    kick_enemies: Field,
    kick_doors: Field,
    difficulty: Field,
    fast_mode: Field,
    auto_move: Field,
}
macro_rules! getter {
    ($name:ident, $out:ty) => {
        pub fn $name(&self) -> $out {
            (*self.$name).try_into().unwrap()
        }
    };
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
                    if line.is_empty() {
                        continue;
                    }
                    let mut iter = line.split(' ');
                    if let Some(field) = iter.next()
                        && let Some(value) = iter.next()
                    {
                        macro_rules! thing {
                            ($field:ident, $val:ty) => {
                                *settings.$field = match value.parse() {
                                    Ok(field) => {
                                        let val: $val = field;
                                        val.into()
                                    }
                                    Err(_) => continue,
                                }
                            };
                        }
                        match field {
                            "kick_enemies" => thing!(kick_enemies, bool),
                            "kick_doors" => thing!(kick_doors, bool),
                            "difficulty" => {
                                difficulty_was_not_set = false;
                                thing!(difficulty, Difficulty)
                            }
                            "fast_mode" => thing!(fast_mode, bool),
                            "auto_move" => thing!(auto_move, bool),
                            _ => {}
                        }
                    }
                }
                if difficulty_was_not_set {
                    *settings.difficulty = Value::from(*difficulty_choice);
                }
                crate::log!("Settings chosen: {settings:?}");
                settings
            }
            false => {
                std::fs::write("settings", DEFAULT_FILE).unwrap();
                let mut settings = Settings::default();
                *settings.difficulty = Value::from(*difficulty_choice);
                settings
            }
        }
    }
    fn to_file(&self) {
        let mut file = std::fs::File::create("settings").unwrap();
        writeln!(
            file,
            "kick_enemies {}",
            bool::try_from(*self.kick_enemies).unwrap()
        )
        .unwrap();
        writeln!(
            file,
            "kick_doors {}",
            bool::try_from(*self.kick_doors).unwrap()
        )
        .unwrap();
        writeln!(
            file,
            "difficulty {}",
            Difficulty::try_from(*self.difficulty).unwrap()
        )
        .unwrap();
        writeln!(
            file,
            "fast_mode {}",
            bool::try_from(*self.fast_mode).unwrap()
        )
        .unwrap();
        writeln!(
            file,
            "auto_move {}",
            bool::try_from(*self.auto_move).unwrap()
        )
        .unwrap();

        file.flush().unwrap();
    }
    fn get_field_mut(&mut self, index: usize) -> &mut Field {
        match index {
            0 => &mut self.kick_enemies,
            1 => &mut self.kick_doors,
            2 => &mut self.difficulty,
            3 => &mut self.fast_mode,
            4 => &mut self.auto_move,
            _ => panic!("I diddly done goofed up the math"),
        }
    }
    fn get_field(&self, index: usize) -> &Field {
        match index {
            0 => &self.kick_enemies,
            1 => &self.kick_doors,
            2 => &self.difficulty,
            3 => &self.fast_mode,
            4 => &self.auto_move,
            _ => panic!("Someone is bad at math, and it is probably me"),
        }
    }
    const fn num_fields(&self) -> usize {
        5
    }

    getter!(kick_enemies, bool);
    getter!(kick_doors, bool);
    getter!(difficulty, Difficulty);
    getter!(fast_mode, bool);
    getter!(auto_move, bool);
}
// Unchanging editor methods
impl Settings {
    // Assumes weirdifier is active
    pub fn editor(&mut self) {
        crossterm::execute!(std::io::stdout(), crossterm::cursor::Hide).unwrap();
        let mut stdin = std::io::stdin().lock();
        let mut buf = [0];
        let num_fields = self.num_fields();
        let mut field_index = 0;
        let mut value_index;
        let mut selecting_field = true;
        loop {
            value_index = self.get_field(field_index).get_index();
            self.editor_render(field_index, value_index, selecting_field, 50);
            stdin.read_exact(&mut buf).unwrap();
            // Add in wasd support
            match buf[0] {
                27 => {
                    stdin.read_exact(&mut buf).unwrap();
                    stdin.read_exact(&mut buf).unwrap();
                    match buf[0] {
                        // Up
                        b'A' => self.editor_up(
                            selecting_field,
                            &mut field_index,
                            num_fields,
                            &mut value_index,
                        ),
                        // Down
                        b'B' => self.editor_down(
                            selecting_field,
                            &mut field_index,
                            num_fields,
                            &mut value_index,
                        ),
                        // Right
                        b'C' => Settings::editor_right(&mut selecting_field),
                        // Left
                        b'D' => Settings::editor_left(&mut selecting_field),
                        _ => {}
                    }
                }
                b'w' => self.editor_up(
                    selecting_field,
                    &mut field_index,
                    num_fields,
                    &mut value_index,
                ),
                b's' => self.editor_down(
                    selecting_field,
                    &mut field_index,
                    num_fields,
                    &mut value_index,
                ),
                b'a' => Self::editor_left(&mut selecting_field),
                b'd' => Self::editor_right(&mut selecting_field),
                b' ' => selecting_field ^= true, // swap selection
                b'q' => break,
                b'Q' => return,
                b'r' => {
                    // reset field
                    self.get_field_mut(field_index).current =
                        Settings::default().get_field(field_index).current;
                }
                b'R' => {
                    // reset all fields
                    *self = Settings::default();
                }
                _ => {}
            }
        }
        self.to_file();
    }
    fn editor_render(
        &self,
        field_index: usize,
        value_index: usize,
        selecting_field: bool,
        second_column: u16,
    ) {
        let selected = *crate::Style::new()
            .background_red()
            .intense_background(true);
        let off_selected = *crate::Style::new().background_yellow();
        let mut lock = std::io::stdout().lock();
        // Clear the screen & zero cursor
        crossterm::queue!(
            lock,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0),
        )
        .unwrap();

        // Drawing fields
        for index in 0..self.num_fields() {
            if index == field_index {
                if selecting_field {
                    write!(lock, "{selected}").unwrap();
                } else {
                    write!(lock, "{off_selected}").unwrap();
                }
            }
            writeln!(lock, "{}", self.get_field(index).name).unwrap();
            if index == field_index {
                write!(lock, "\x1b[0m").unwrap();
            }
        }

        // Drawing values
        let values = self.get_field(field_index).get_values();
        for (index, value) in values.iter().enumerate() {
            crossterm::queue!(lock, crossterm::cursor::MoveTo(second_column, index as u16))
                .unwrap();
            if index == value_index {
                if selecting_field {
                    write!(lock, "{off_selected}").unwrap();
                } else {
                    write!(lock, "{selected}").unwrap();
                }
            }
            write!(lock, "{value}").unwrap();
            if index == value_index {
                write!(lock, "\x1b[0m").unwrap();
            }
        }

        crossterm::queue!(lock, crossterm::cursor::MoveTo(0, 10000)).unwrap();
        write!(
            lock,
            "q to quit, Q to quit without saving, r to reset field, R to full reset"
        )
        .unwrap();

        lock.flush().unwrap();
    }
    fn editor_up(
        &mut self,
        selecting_field: bool,
        field_index: &mut usize,
        num_fields: usize,
        value_index: &mut usize,
    ) {
        match selecting_field {
            true => {
                if *field_index == 0 {
                    *field_index = num_fields;
                }
                *field_index -= 1;
            }
            false => {
                if *value_index == 0 {
                    *value_index = self.get_field(*field_index).get_values().len();
                }
                *value_index -= 1;
                self.get_field_mut(*field_index).current =
                    self.get_field(*field_index).get_values()[*value_index];
            }
        }
    }
    fn editor_down(
        &mut self,
        selecting_field: bool,
        field_index: &mut usize,
        num_fields: usize,
        value_index: &mut usize,
    ) {
        match selecting_field {
            true => {
                *field_index += 1;
                if *field_index == num_fields {
                    *field_index = 0;
                }
            }
            false => {
                *value_index += 1;
                if *value_index == self.get_field(*field_index).get_values().len() {
                    *value_index = 0;
                }
                self.get_field_mut(*field_index).current =
                    self.get_field(*field_index).get_values()[*value_index];
            }
        }
    }
    fn editor_left(selecting_field: &mut bool) {
        *selecting_field = true;
    }
    fn editor_right(selecting_field: &mut bool) {
        *selecting_field = false;
    }
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            kick_enemies: Field::new("kick enemies", true),
            kick_doors: Field::new("kick doors", true),
            difficulty: Field::new("difficulty", Difficulty::default()),
            fast_mode: Field::new("fast mode", false),
            auto_move: Field::new("auto move", false),
        }
    }
}
impl FromBinary for Settings {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Settings {
            kick_enemies: Value::from(bool::from_binary(binary)?).into(),
            kick_doors: Value::from(bool::from_binary(binary)?).into(),
            difficulty: Value::from(Difficulty::from_binary(binary)?).into(),
            fast_mode: Value::from(bool::from_binary(binary)?).into(),
            auto_move: Value::from(bool::from_binary(binary)?).into(),
        })
    }
}
impl ToBinary for Settings {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        macro_rules! help {
            ($field:ident, $type:ty) => {
                <$type>::try_from(*self.$field).unwrap().to_binary(binary)?;
            };
        }
        help!(kick_enemies, bool);
        help!(kick_doors, bool);
        help!(difficulty, Difficulty);
        help!(fast_mode, bool);
        help!(auto_move, bool);
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct Field {
    current: Value,
    name: &'static str,
}
impl Field {
    fn new<T: Into<Value>>(name: &'static str, default: T) -> Self {
        Field {
            current: default.into(),
            name,
        }
    }
}
impl From<Value> for Field {
    // If we are loading using FromBinary then the editor is not running,
    // therefore we only need the current value.
    fn from(value: Value) -> Self {
        Field {
            current: value,
            name: "",
        }
    }
}
impl std::ops::Deref for Field {
    type Target = Value;
    fn deref(&self) -> &Self::Target {
        &self.current
    }
}
impl std::ops::DerefMut for Field {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Difficulty(Difficulty),
}
impl Value {
    fn get_values(&self) -> Vec<Value> {
        match self {
            Value::Bool(_) => Vec::from([true, false].map(Value::Bool)),
            Value::Difficulty(_) => Vec::from(
                [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard].map(Value::Difficulty),
            ),
        }
    }
    fn get_index(&self) -> usize {
        for (index, value) in self.get_values().iter().enumerate() {
            if value == self {
                return index;
            }
        }
        unreachable!("ABE YA STUPID GIT IT DIDN'T WORK")
    }
}
impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}
impl From<Difficulty> for Value {
    fn from(value: Difficulty) -> Self {
        Value::Difficulty(value)
    }
}
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(val) => val.fmt(f),
            Value::Difficulty(val) => val.fmt(f),
        }
    }
}
impl TryFrom<Value> for bool {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Bool(val) = value {
            Ok(val)
        } else {
            Err(())
        }
    }
}
impl TryFrom<Value> for Difficulty {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Difficulty(val) = value {
            Ok(val)
        } else {
            Err(())
        }
    }
}
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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
        Some(self.cmp(other))
    }
}
impl Ord for Difficulty {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self == other {
            return std::cmp::Ordering::Equal;
        }
        match self {
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
        }
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
