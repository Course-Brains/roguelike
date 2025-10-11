use std::io::Write;

use crate::{FromBinary, Style, ToBinary, player::Duration, spell::NormalSpell};

const SCROLL: char = 'S';
const POTION: char = 'P';

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ItemType {
    // regen effect 10 turns
    HealthPotion,
    // Finds the boss/exit
    BossFinder,
    // Half or double money
    Gamba,
    // It teleports you to where you throw it
    EnderPearl,
    // Teleport to wherever you want, assuming no enemies and no collision
    Warp,
    // makes you invincible "I'm holdin' tea!" - Em
    Tea,
    // A "spirit"
    Spirit,
    // gives far_sight for 50 turns
    FarSightPotion,
    // Gives immediate health but poisons you
    Fish,
}
impl ItemType {
    // What is listed in the inventory
    pub fn name(self, out: &mut impl std::io::Write) {
        match self {
            Self::HealthPotion => write!(out, "Potion of healing").unwrap(),
            Self::BossFinder => write!(out, "Scroll of seeking").unwrap(),
            Self::Gamba => write!(out, "Scroll of chance").unwrap(),
            Self::EnderPearl => write!(out, "Scroll of teleportation").unwrap(),
            Self::Warp => write!(out, "Scroll of warping").unwrap(),
            Self::Tea => write!(out, "Tea").unwrap(),
            Self::Spirit => write!(out, "Spirit").unwrap(),
            Self::FarSightPotion => write!(out, crate::debug_only!("far_sight_potion")).unwrap(),
            Self::Fish => write!(out, "fish").unwrap(),
        }
    }
    // What happens when it is used
    // returns whether or not it succeeded and should take the turn
    pub fn enact(self, state: &mut crate::State) -> bool {
        match self {
            Self::HealthPotion => {
                state.player.effects.regen += 11;
                true
            }
            Self::BossFinder => {
                let player_pos = state.player.pos;
                let mut min_dist = usize::MAX;
                let mut min_pos = crate::Vector::new(0, 0);
                for boss_pos in state
                    .board
                    .bosses
                    .iter()
                    .filter(|boss| boss.sibling.upgrade().is_some())
                    .map(|boss| boss.last_pos)
                {
                    let dist = boss_pos.abs_diff(player_pos).sum_axes();
                    if dist < min_dist {
                        min_dist = dist;
                        min_pos = boss_pos;
                    }
                }
                if min_dist != usize::MAX {
                    crate::set_feedback(format!("The nearest boss is at {min_pos}"));
                    true
                } else {
                    crate::set_feedback("There are no remaining bosses".to_string());
                    false
                }
            }
            Self::Gamba => {
                let random = crate::random();
                if random == 0 {
                    // super good luck
                    *crate::feedback() = "You feel blessed".to_string();
                    if state.player.effects.doomed.is_active() {
                        state.player.effects.doomed.remove();
                    } else if state.player.effects.unlucky.is_active() {
                        state.player.effects.unlucky.remove();
                    } else {
                        state.player.max_health += state.player.max_health / 2;
                        state.player.max_energy += state.player.max_energy / 2;
                        state.player.give_money(state.player.get_money());
                        state.player.perception += 5;
                    }
                    return true;
                }
                let mut already_said = false;
                if random & 0b0011_1000 == 0 {
                    already_said = true;
                    match (random & 0b1100_0000) >> 6 {
                        0 => {
                            // good luck: cure unlucky
                            if state.player.effects.unlucky.is_active() {
                                *crate::feedback() =
                                    "You feel a wave of relief wash over you".to_string();
                                state.player.effects.unlucky = Duration::None;
                            } else {
                                *crate::feedback() =
                                    "Nothing happens but you feel good anyway".to_string();
                            }
                        }
                        1 => {
                            // bad luck: become unlucky
                            if !state.player.effects.unlucky.is_active() {
                                *crate::feedback() = "You don't feel quite right".to_string();
                                state.player.effects.unlucky = Duration::Infinite;
                            } else {
                                *crate::feedback() = "The scroll sympathizes with you".to_string();
                            }
                        }
                        2 => {
                            // good luck: increase perception
                            *crate::feedback() =
                                "Your eyes seem a bit better than before".to_string();
                            state.player.perception += 3;
                        }
                        3 => {
                            // bad luck: become more detectable
                            *crate::feedback() = "You feel clumsy".to_string();
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
                        *crate::feedback() = "Your luck proves benefitial".to_string();
                    }
                    state.player.give_money(state.player.get_money());
                } else {
                    if !already_said {
                        *crate::feedback() = "Your luck proves detrimental".to_string();
                    }
                    state.player.take_money(state.player.get_money() / 2);
                }
                true
            }
            Self::EnderPearl => {
                let aim = state.player.selector;
                NormalSpell::Teleport.cast(
                    None,
                    &mut state.player,
                    &mut state.board,
                    None,
                    Some(aim),
                    None,
                );
                true
            }
            Self::Warp => {
                if state.board[state.player.selector]
                    .as_ref()
                    .is_none_or(|piece| !piece.has_collision())
                    && state.board.get_enemy(state.player.selector, None).is_none()
                {
                    state.player.pos = state.player.selector;
                    crate::re_flood();
                    return true;
                }
                *crate::feedback() = "Failed to warp to the target location".to_string();
                state.reposition_cursor();
                crate::bell(None);
                false
            }
            Self::Tea => {
                state.player.effects.drunk.remove();
                state.player.effects.invincible.increase_to(10, 20);
                state.player.effects.damage_boost.increase_to(40, 80);
                true
            }
            Self::Spirit => {
                state.player.effects.drunk += 50;
                true
            }
            Self::FarSightPotion => {
                state.player.effects.far_sight += 20;
                true
            }
            Self::Fish => {
                state.player.heal(20);
                state.player.effects.poison *= 2;
                state.player.effects.poison += 10;
                true
            }
        }
    }
    // The price to pick up
    pub fn price(self) -> usize {
        match self {
            Self::HealthPotion => 10,
            Self::BossFinder => 30,
            Self::Gamba => 15,
            Self::EnderPearl => 15,
            Self::Warp => 30,
            Self::Tea => 50,
            Self::FarSightPotion => 15,
            Self::Fish => 35,
            Self::Spirit => unimplemented!("Spirit intentionally not in shop"),
        }
    }
    // What is said when on the ground
    pub fn get_desc(self) -> &'static str {
        match self {
            Self::HealthPotion => "Potion of healing",
            Self::BossFinder => "Scroll of seeking",
            Self::Gamba => "Scroll of chance",
            Self::EnderPearl => "Scroll of teleportation",
            Self::Warp => "Scroll of warping",
            Self::Tea => "Tea",
            Self::Spirit => "Spirit",
            Self::FarSightPotion => crate::debug_only!("far_sight_potion"),
            Self::Fish => "fish",
        }
    }
    pub fn render(self, player: &crate::Player) -> (char, Option<Style>) {
        (
            match self {
                Self::BossFinder | Self::Gamba | Self::EnderPearl | Self::Warp | Self::Fish => {
                    SCROLL
                }
                Self::HealthPotion | Self::Tea | Self::Spirit | Self::FarSightPotion => POTION,
            },
            Some(match self.price() <= player.get_money() {
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
            "health_potion" => Ok(Self::HealthPotion),
            "boss_finder" => Ok(Self::BossFinder),
            "gamba" => Ok(Self::Gamba),
            "ender_pearl" => Ok(Self::EnderPearl),
            "warp" => Ok(Self::Warp),
            "tea" => Ok(Self::Tea),
            "spirit" => Ok(Self::Spirit),
            "far_sight_potion" => Ok(Self::FarSightPotion),
            "fish" => Ok(Self::Fish),
            other => Err(format!("{other} is not an item type")),
        }
    }
}
impl std::fmt::Display for ItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::HealthPotion => write!(f, "health potion"),
            Self::BossFinder => write!(f, "boss finder"),
            Self::Gamba => write!(f, "gamba"),
            Self::EnderPearl => write!(f, "ender pearl"),
            Self::Warp => write!(f, "warp"),
            Self::Tea => write!(f, "tea"),
            Self::Spirit => write!(f, "spirit"),
            Self::FarSightPotion => write!(f, "far sight potion"),
            Self::Fish => write!(f, "fish"),
        }
    }
}
impl crate::Random for crate::ItemType {
    fn random() -> Self {
        match crate::random() % 8 {
            0 => Self::HealthPotion,
            1 => Self::BossFinder,
            2 => Self::Gamba,
            3 => Self::EnderPearl,
            4 => Self::Warp,
            5 => Self::Tea,
            6 => Self::FarSightPotion,
            7 => Self::Fish,
            // Spirit intentionally not in shop
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
            0 => Self::HealthPotion,
            1 => Self::BossFinder,
            2 => Self::Gamba,
            3 => Self::EnderPearl,
            4 => Self::Warp,
            5 => Self::Tea,
            6 => Self::Spirit,
            7 => Self::FarSightPotion,
            8 => Self::Fish,
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
            Self::HealthPotion => 0_u8,
            Self::BossFinder => 1,
            Self::Gamba => 2,
            Self::EnderPearl => 3,
            Self::Warp => 4,
            Self::Tea => 5,
            Self::Spirit => 6,
            Self::FarSightPotion => 7,
            Self::Fish => 8,
        }
        .to_binary(binary)
    }
}
