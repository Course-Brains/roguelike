use crate::{Player, Random, Style, player::Duration, random};

const SYMBOL: char = 'U';
const AVAILABLE: Style = *Style::new().green();
const UNAVAILABLE: Style = *Style::new().red();

#[derive(Clone, Copy, Debug)]
pub struct Upgrades {
    pub mage_eye: usize,
    pub map: bool,
    pub soft_shoes: bool,
}
impl Upgrades {
    pub const fn new() -> Upgrades {
        Upgrades {
            mage_eye: 0,
            map: false,
            soft_shoes: false,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum UpgradeType {
    MageEye,
    Map,
    SoftShoes,
}
impl UpgradeType {
    pub fn render(&self, player: &Player) -> (char, Option<Style>) {
        (
            SYMBOL,
            Some(
                match self.cost() <= player.money && self.can_pickup(player) {
                    true => AVAILABLE,
                    false => UNAVAILABLE,
                },
            ),
        )
    }
    pub fn cost(self) -> usize {
        match self {
            Self::MageEye => 200,
            Self::Map => 300,
            Self::SoftShoes => 200,
        }
    }
    pub fn on_pickup(self, player: &mut Player) {
        match self {
            Self::MageEye => {
                player.effects.mage_sight = Duration::Infinite;
                player.upgrades.mage_eye += 1;
                let _ = player.attacked(((random() as usize & 3) + 1) * 5, "stupidity");
            }
            Self::Map => {
                player.upgrades.map = true;
            }
            Self::SoftShoes => {
                player.detect_mod -= 1;
                player.upgrades.soft_shoes = true
            }
        }
    }
    pub fn can_pickup(self, player: &Player) -> bool {
        match self {
            Self::MageEye => player.upgrades.mage_eye < 2,
            Self::Map => !player.upgrades.map,
            Self::SoftShoes => !player.upgrades.soft_shoes,
        }
    }
    pub fn get_desc(self) -> &'static str {
        match self {
            Self::MageEye => "A mage's eye",
            Self::Map => "A map",
            Self::SoftShoes => "A pair of particularly soft shoes",
        }
    }
}
impl std::fmt::Display for UpgradeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MageEye => write!(f, "mage eye"),
            Self::Map => write!(f, "map"),
            Self::SoftShoes => write!(f, "soft shoes"),
        }
    }
}
impl std::str::FromStr for UpgradeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mage_eye" => Ok(Self::MageEye),
            "map" => Ok(Self::Map),
            "soft_shoes" => Ok(Self::SoftShoes),
            other => Err(format!("{other} is not a valid upgrade")),
        }
    }
}
impl Random for UpgradeType {
    fn random() -> Self {
        match random() & 0b0000_0001 {
            0 => Self::MageEye,
            1 => Self::Map,
            _ => unreachable!("Le fucked is up"),
        }
    }
}
