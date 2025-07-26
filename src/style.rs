use crate::{FromBinary, ToBinary};
#[derive(Clone, Copy)]
pub struct Style {
    color: Color,
    intense: bool,
    background: Color,
    intense_background: bool,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    strike: bool,
}
macro_rules! color {
    ($background: ident, $name:ident, $variant:ident) => {
        pub const fn $name(&mut self) -> &mut Self {
            self.color = Color::$variant;
            self
        }
        pub const fn $background(&mut self) -> &mut Self {
            self.background = Color::$variant;
            self
        }
    };
}
macro_rules! set {
    ($name: ident) => {
        pub const fn $name(&mut self, $name: bool) -> &mut Self {
            self.$name = $name;
            self
        }
    };
}
impl Style {
    pub const fn new() -> Style {
        Style {
            color: Color::Default,
            intense: false,
            background: Color::Default,
            intense_background: false,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            strike: false,
        }
    }
    pub const fn set_color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }
    pub const fn set_background(&mut self, color: Color) -> &mut Self {
        self.background = color;
        self
    }
    pub const fn has_background(&self) -> bool {
        if let Color::Default = self.background {
            return false;
        }
        true
    }

    set!(intense);
    set!(intense_background);
    set!(bold);
    set!(dim);
    set!(italic);
    set!(underline);
    set!(strike);

    color!(background_black, black, Black);
    color!(background_red, red, Red);
    color!(background_green, green, Green);
    color!(background_yellow, yellow, Yellow);
    color!(background_blue, blue, Blue);
    color!(background_purple, purple, Purple);
    color!(background_cyan, cyan, Cyan);
    color!(background_white, white, White);
    color!(background_default, default, Default);
}
impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut color = 0;
        if let Some(num) = self.color.to_num() {
            color = num;
            if self.intense {
                color += 60;
            }
        }
        if self.bold {
            write!(f, "\x1b[1m")?;
        }
        if self.dim {
            write!(f, "\x1b[2m")?;
        }
        if self.italic {
            write!(f, "\x1b[3m")?;
        }
        if self.underline {
            write!(f, "\x1b[4m")?;
        }
        if self.strike {
            write!(f, "\x1b[9m")?;
        }
        match self.background.to_num() {
            Some(mut background) => {
                if self.intense_background {
                    background += 60
                }
                background += 10;
                write!(f, "\x1b[0;{};{}m", color, background)
            }
            None => {
                write!(f, "\x1b[0;{}m", color)
            }
        }
    }
}
impl FromBinary for Style {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Style {
            color: Color::from_binary(binary)?,
            intense: bool::from_binary(binary)?,
            background: Color::from_binary(binary)?,
            intense_background: bool::from_binary(binary)?,
            bold: bool::from_binary(binary)?,
            dim: bool::from_binary(binary)?,
            italic: bool::from_binary(binary)?,
            underline: bool::from_binary(binary)?,
            strike: bool::from_binary(binary)?,
        })
    }
}
impl ToBinary for Style {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        self.color.to_binary(binary)?;
        self.intense.to_binary(binary)?;
        self.background.to_binary(binary)?;
        self.intense_background.to_binary(binary)?;
        self.bold.to_binary(binary)?;
        self.dim.to_binary(binary)?;
        self.italic.to_binary(binary)?;
        self.underline.to_binary(binary)?;
        self.strike.to_binary(binary)
    }
}
#[derive(Clone, Copy)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Cyan,
    White,
    Default,
}
impl Color {
    fn to_num(self) -> Option<u8> {
        match self {
            Color::Black => Some(30),
            Color::Red => Some(31),
            Color::Green => Some(32),
            Color::Yellow => Some(33),
            Color::Blue => Some(34),
            Color::Purple => Some(35),
            Color::Cyan => Some(36),
            Color::White => Some(37),
            Color::Default => None,
        }
    }
}
impl FromBinary for Color {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Purple,
            6 => Color::Cyan,
            7 => Color::White,
            8 => Color::Default,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not get Color from binary",
                ));
            }
        })
    }
}
impl ToBinary for Color {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        match self {
            Color::Black => 0_u8,
            Color::Red => 1_u8,
            Color::Green => 2_u8,
            Color::Yellow => 3_u8,
            Color::Blue => 4_u8,
            Color::Purple => 5_u8,
            Color::Cyan => 6_u8,
            Color::White => 7_u8,
            Color::Default => 8_u8,
        }
        .to_binary(binary)
    }
}
