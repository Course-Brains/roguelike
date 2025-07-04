use crate::Style;

const SCROLL: char = 'S';
const POTION: char = 'P';

#[derive(Clone, Copy, Debug)]
pub enum ItemType {
    // mage sight effect 100 turns
    MageSight,
    // regen effect 10 turns
    HealthPotion,
}
impl ItemType {
    // What is listed in the inventory
    pub fn name(self, out: &mut impl std::io::Write) {
        match self {
            Self::MageSight => write!(out, "Scroll of magical sight").unwrap(),
            Self::HealthPotion => write!(out, "Potion of healing").unwrap(),
        }
    }
    // What happens when it is used
    // returns whether or not it succeeded and should take the turn
    pub fn enact(self, state: &mut crate::State) -> bool {
        match self {
            Self::MageSight => {
                if state.player.effects.mage_sight.is_active() {
                    state.player.effects.mage_sight += 50;
                } else {
                    state.player.effects.mage_sight += 100;
                }
                true
            }
            Self::HealthPotion => {
                state.player.effects.regen += 11;
                true
            }
        }
    }
    // The price to pick up
    pub fn price(self) -> usize {
        match self {
            Self::MageSight => 5,
            Self::HealthPotion => 10,
        }
    }
    // What is said when on the ground
    pub fn get_desc(self) -> &'static str {
        match self {
            Self::MageSight => "Scroll of magical sight",
            Self::HealthPotion => "Potion of healing",
        }
    }
    pub fn render(self, player: &crate::Player) -> (char, Option<Style>) {
        (
            match self {
                Self::MageSight => SCROLL,
                Self::HealthPotion => POTION,
            },
            Some(match self.price() <= player.money {
                true => *Style::new().green(),
                false => *Style::new().red(),
            }),
        )
    }
}
impl std::str::FromStr for ItemType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mage_sight" => Ok(Self::MageSight),
            "health_potion" => Ok(Self::HealthPotion),
            other => Err(format!("{other} is not an item type")),
        }
    }
}
impl std::fmt::Display for ItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::MageSight => write!(f, "mage sight"),
            Self::HealthPotion => write!(f, "health potion"),
        }
    }
}
