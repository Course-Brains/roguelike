#[derive(Clone, Copy)]
pub struct Style {
    color: Color,
    intense: bool,
    background: Color
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
            background: Color::Default
        }
    }
    // assumes we start from [0m
    pub fn enact(&self) -> String {
        match self.color.to_num() {
            Some(mut num) => {
                if self.intense {
                    num += 60;
                }
                std::fmt::format(format_args!("\x1b[0;{}m", num))
            },
            None => "".to_string()
        }
    }
    pub const fn intense(mut self, intense: bool) -> Self {
        self.intense = intense;
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
