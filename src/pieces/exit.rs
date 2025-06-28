use crate::{Style, pieces::spell::Stepper};
use std::sync::atomic::Ordering;
pub const SYMBOL: char = 'Π';
pub const STYLE: Style = Style::new();
#[derive(Clone, Copy, Debug)]
pub enum Exit {
    Shop,
    Level,
}
impl Exit {
    pub const fn render() -> (char, Option<Style>) {
        (SYMBOL, Some(STYLE))
    }
    pub fn on_step(self, stepper: Stepper<'_>) {
        if let Stepper::Player(_) = stepper {
            match self {
                Exit::Shop => crate::LOAD_SHOP.store(true, Ordering::Relaxed),
                Exit::Level => crate::LOAD_MAP.store(true, Ordering::Relaxed),
            }
        }
    }
}
impl std::fmt::Display for Exit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exit::Shop => write!(f, "Exit to shop"),
            Exit::Level => write!(f, "Exit to next level"),
        }
    }
}
impl std::str::FromStr for Exit {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "shop" => Ok(Exit::Shop),
            "level" => Ok(Exit::Level),
            invalid => Err(format!("{invalid} is not shop or level")),
        }
    }
}
