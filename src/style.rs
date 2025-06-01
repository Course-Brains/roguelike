#[derive(Clone, Copy)]
pub struct Style {
    color: Color,
    intense: bool,
    background: Color,
    intense_background: bool
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
    }
}
impl Style {
    pub const fn new() -> Style {
        Style {
            color: Color::Default,
            intense: false,
            background: Color::Default,
            intense_background: false
        }
    }
    // assumes we start from [0m
    pub fn enact(&self) -> String {
        let mut color = 0;
        if let Some(num) = self.color.to_num() {
            color = num;
            if self.intense {
                color += 60;
            }
        }
        match self.background.to_num() {
            Some(mut background) => {
                if self.intense_background {
                    background += 60
                }
                background += 10;
                format!("\x1b[0;{};{}m", color, background)
            }
            None => {
                format!("\x1b[0;{}m", color)
            }
        }
    }
    pub const fn has_background(&self) -> bool {
        if let Color::Default = self.background {
            return false
        }
        true
    }
    pub const fn intense(&mut self, intense: bool) -> &mut Self {
        self.intense = intense;
        self
    }
    pub const fn intense_background(&mut self, intense: bool) -> &mut Self {
        self.intense_background = intense;
        self
    }
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
    Default
}
impl Color {
    fn to_num(self) -> Option<u8>{
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
