use crate::{Entity, FromBinary, Style, ToBinary};
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
    pub fn on_step(self, stepper: Entity<'_>) {
        if let Entity::Player(_) = stepper {
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
impl FromBinary for Exit {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match bool::from_binary(binary)? {
            true => Exit::Shop,
            false => Exit::Level,
        })
    }
}
impl ToBinary for Exit {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        match self {
            Exit::Shop => true.to_binary(binary),
            Exit::Level => false.to_binary(binary),
        }
    }
}
