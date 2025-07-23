use std::io::Write;

use crate::{FromBinary, Style, ToBinary, player::Duration};

const SCROLL: char = 'S';
const POTION: char = 'P';

#[derive(Clone, Copy, Debug)]
pub enum ItemType {
    // mage sight effect 100 turns
    MageSight,
    // regen effect 10 turns
    HealthPotion,
    // Finds the boss/exit
    BossFinder,
    // Half or double money
    Gamba,
}
impl ItemType {
    // What is listed in the inventory
    pub fn name(self, out: &mut impl std::io::Write) {
        match self {
            Self::MageSight => write!(out, "Scroll of magical sight").unwrap(),
            Self::HealthPotion => write!(out, "Potion of healing").unwrap(),
            Self::BossFinder => write!(out, "Scroll of locate target").unwrap(),
            Self::Gamba => write!(out, "Scroll of chance").unwrap(),
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
            Self::BossFinder => {
                let target = match state
                    .board
                    .boss
                    .as_ref()
                    .is_some_and(|boss| boss.upgrade().is_some())
                {
                    true => "boss",
                    false => "exit",
                };
                state.player.inspect = false;
                crate::Board::set_desc(
                    &mut std::io::stdout(),
                    format!("the {} is at {}", target, state.board.boss_pos).as_str(),
                );
                std::io::stdout().flush().unwrap();
                true
            }
            Self::Gamba => {
                let random = crate::random();
                if random == 0 {
                    // super good luck
                    crate::set_desc("You feel blessed");
                    if state.player.effects.doomed.is_active() {
                        state.player.effects.doomed.remove();
                    } else if state.player.effects.unlucky.is_active() {
                        state.player.effects.unlucky.remove();
                    } else {
                        state.player.max_health += state.player.max_health / 2;
                        state.player.max_energy += state.player.max_energy / 2;
                        state.player.money *= 2;
                        state.player.perception += 5;
                    }
                    return true;
                }
                state.player.inspect = false;
                let mut already_said = false;
                if random & 0b0011_1000 == 0 {
                    already_said = true;
                    match (random & 0b1100_0000) >> 6 {
                        0 => {
                            // good luck: cure unlucky
                            if state.player.effects.unlucky.is_active() {
                                crate::set_desc("You feel a wave of relief wash over you");
                                state.player.effects.unlucky = Duration::None;
                            } else {
                                crate::set_desc("Nothing happens but you feel good anyway");
                            }
                        }
                        1 => {
                            // bad luck: become unlucky
                            if !state.player.effects.unlucky.is_active() {
                                crate::set_desc("You don't feel quite right");
                                state.player.effects.unlucky = Duration::Infinite;
                            } else {
                                crate::set_desc("The scroll sympathizes with you");
                            }
                        }
                        2 => {
                            // good luck: increase perception
                            crate::set_desc("Your eyes seem a bit better than before");
                            state.player.perception += 3;
                        }
                        3 => {
                            // bad luck: become more detectable
                            crate::set_desc("You feel clumsy");
                            state.player.detect_mod += 1;
                        }
                        _ => unreachable!("I did a fucky wucky"),
                    }
                }
                // uses last three bits
                let chance;
                if state.player.effects.doomed.is_active() {
                    chance = 0b0000_0111;
                } else if state.player.effects.unlucky.is_active() {
                    chance = 0b0000_0011;
                } else {
                    chance = 0b0000_0001;
                }
                if random & chance == 0 {
                    if !already_said {
                        crate::set_desc("Your luck proves benefitial")
                    }
                    state.player.money *= 2;
                } else {
                    if !already_said {
                        crate::set_desc("Your luck proves detrimental")
                    }
                    state.player.money /= 2;
                }
                true
            }
        }
    }
    // The price to pick up
    pub fn price(self) -> usize {
        match self {
            Self::MageSight => 5,
            Self::HealthPotion => 10,
            Self::BossFinder => 30,
            Self::Gamba => 15,
        }
    }
    // What is said when on the ground
    pub fn get_desc(self) -> &'static str {
        match self {
            Self::MageSight => "Scroll of magical sight",
            Self::HealthPotion => "Potion of healing",
            Self::BossFinder => "Scroll of locate target",
            Self::Gamba => "Scroll of chance",
        }
    }
    pub fn render(self, player: &crate::Player) -> (char, Option<Style>) {
        (
            match self {
                Self::MageSight => SCROLL,
                Self::HealthPotion => POTION,
                Self::BossFinder => SCROLL,
                Self::Gamba => SCROLL,
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
            "boss_finder" => Ok(Self::BossFinder),
            "gamba" => Ok(Self::Gamba),
            other => Err(format!("{other} is not an item type")),
        }
    }
}
impl std::fmt::Display for ItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::MageSight => write!(f, "mage sight"),
            Self::HealthPotion => write!(f, "health potion"),
            Self::BossFinder => write!(f, "boss finder"),
            Self::Gamba => write!(f, "gamba"),
        }
    }
}
impl crate::Random for crate::ItemType {
    fn random() -> Self {
        match crate::random() & 0b0000_0011 {
            0 => Self::MageSight,
            1 => Self::HealthPotion,
            2 => Self::BossFinder,
            3 => Self::Gamba,
            _ => unreachable!("idk, not my problem"),
        }
    }
}
impl FromBinary for ItemType {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Self::MageSight,
            1 => Self::HealthPotion,
            2 => Self::BossFinder,
            3 => Self::Gamba,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not get ItemType from binary",
                ));
            }
        })
    }
}
impl ToBinary for ItemType {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        match self {
            Self::MageSight => 0_u8.to_binary(binary),
            Self::HealthPotion => 1_u8.to_binary(binary),
            Self::BossFinder => 2_u8.to_binary(binary),
            Self::Gamba => 3_u8.to_binary(binary),
        }
    }
}
