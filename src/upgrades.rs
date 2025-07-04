use crate::{Player, Style, player::Duration};

const SYMBOL: char = 'U';
const AVAILABLE: Style = *Style::new().green();
const UNAVAILABLE: Style = *Style::new().red();

#[derive(Clone, Copy, Debug)]
pub struct Upgrades {
    mage_eye: usize,
}
impl Upgrades {
    pub const fn new() -> Upgrades {
        Upgrades { mage_eye: 0 }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum UpgradeType {
    MageEye,
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
            Self::MageEye => 300,
        }
    }
    pub fn on_pickup(self, player: &mut Player) {
        match self {
            Self::MageEye => {
                player.effects.mage_sight = Duration::Infinite;
                player.upgrades.mage_eye += 1;
                let _ = player.attacked(20, "stupidity");
            }
        }
    }
    pub fn can_pickup(self, player: &Player) -> bool {
        match self {
            Self::MageEye => player.upgrades.mage_eye < 2,
        }
    }
    pub fn get_desc(self) -> &'static str {
        match self {
            Self::MageEye => "A mage's eye",
        }
    }
}
impl std::fmt::Display for UpgradeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MageEye => write!(f, "mage eye"),
        }
    }
}
impl std::str::FromStr for UpgradeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mage_eye" => Ok(Self::MageEye),
            other => Err(format!("{other} is not a valid upgrade")),
        }
    }
}
