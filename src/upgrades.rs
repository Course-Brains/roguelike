use crate::{FromBinary, Player, Random, Style, ToBinary, get, random};
use std::io::Read;

const AVAILABLE: Style = *Style::new().green();
const UNAVAILABLE: Style = *Style::new().red();

#[derive(Clone, Copy, Debug)]
pub struct Upgrades {
    pub map: bool,
    pub soft_shoes: bool,
    pub precise_convert: bool,
    pub lifesteal: bool,
    pub full_energy_ding: bool,
}
impl Upgrades {
    pub const fn new() -> Upgrades {
        Upgrades {
            map: false,
            soft_shoes: false,
            precise_convert: false,
            lifesteal: false,
            full_energy_ding: false,
        }
    }
    pub fn get_available(&self) -> Vec<UpgradeType> {
        let mut out = vec![
            UpgradeType::EnergyBoost,
            UpgradeType::HealthBoost,
            UpgradeType::LimbMageEye,
            UpgradeType::LimbSeerEye,
        ];
        if !self.map {
            out.push(UpgradeType::Map);
        }
        if !self.soft_shoes {
            out.push(UpgradeType::SoftShoes);
        }
        if !self.precise_convert {
            out.push(UpgradeType::PreciseConvert);
        }
        if !self.full_energy_ding {
            out.push(UpgradeType::FullEnergyDing)
        }
        if !self.lifesteal {
            out.push(UpgradeType::FullEnergyDing)
        }

        out
    }
}
impl FromBinary for Upgrades {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Upgrades {
            map: bool::from_binary(binary)?,
            soft_shoes: bool::from_binary(binary)?,
            precise_convert: bool::from_binary(binary)?,
            lifesteal: bool::from_binary(binary)?,
            full_energy_ding: bool::from_binary(binary)?,
        })
    }
}
impl ToBinary for Upgrades {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        self.map.to_binary(binary)?;
        self.soft_shoes.to_binary(binary)?;
        self.precise_convert.to_binary(binary)?;
        self.lifesteal.to_binary(binary)?;
        self.full_energy_ding.to_binary(binary)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum UpgradeType {
    // Normal upgrades
    Map,            // see all walls
    SoftShoes,      // advantage on stealth
    PreciseConvert, // convert one energy at a time
    FullEnergyDing, // send bell character when full energy
    Lifesteal,      // health on kill
    // Infinite upgrades
    EnergyBoost, // max energy (exponential cost?)
    HealthBoost, // max health ...
    // Limbs
    LimbMageEye, // The mage eye
    LimbSeerEye, // The seer eye
    // Bonuses
    BonusNoWaste,  // double money
    BonusNoDamage, // +50% max health & full heal
    BonusKillAll,  // complicated, look at on_pickup
    BonusNoEnergy, // +50% max energy
    // Save pint
    SavePint, // no corresponding upgrade field
    // Spells
    Spell(crate::spell::Spell), // what grants new spells
}
impl UpgradeType {
    pub fn render(&self, player: &Player) -> (char, Option<Style>) {
        let symbol = match self {
            // P for Pint
            Self::SavePint => 'P',
            // B for Bonus
            Self::BonusNoWaste | Self::BonusNoDamage | Self::BonusKillAll | Self::BonusNoEnergy => {
                'B'
            }
            // L for limb
            Self::LimbMageEye | Self::LimbSeerEye => 'L',
            // S for Spell
            Self::Spell(_) => 'S',
            // U for Upgrade
            _ => 'U',
        };
        (
            symbol,
            Some(
                match self.cost() <= player.get_money() && self.can_pickup(player) {
                    true => AVAILABLE,
                    false => UNAVAILABLE,
                },
            ),
        )
    }
    pub fn cost(self) -> usize {
        match self {
            Self::Map => 300,
            Self::SoftShoes => 200,
            Self::PreciseConvert | Self::Lifesteal => 150,
            Self::EnergyBoost | Self::HealthBoost | Self::LimbMageEye | Self::LimbSeerEye => 100,
            Self::FullEnergyDing => 50,
            Self::Spell(spell) => spell.get_cost(),
            // The free shit
            Self::SavePint
            | Self::BonusNoWaste
            | Self::BonusNoDamage
            | Self::BonusKillAll
            | Self::BonusNoEnergy => 0,
        }
    }
    pub fn on_pickup(self, player: &mut Player) -> bool {
        match self {
            Self::Map => {
                player.upgrades.map = true;
                true
            }
            Self::SoftShoes => {
                player.detect_mod -= 1;
                player.upgrades.soft_shoes = true;
                true
            }
            Self::SavePint => {
                crate::set_desc("Drink the save pint? y/n");
                let mut lock = std::io::stdin().lock();
                let mut buf = [0];
                loop {
                    lock.read_exact(&mut buf).unwrap();
                    match buf[0] {
                        b'y' => {
                            crate::SAVE.store(true, std::sync::atomic::Ordering::Relaxed);
                            break true;
                        }
                        b'n' => break false,
                        _ => {}
                    }
                }
            }
            Self::PreciseConvert => {
                player.upgrades.precise_convert = true;
                true
            }
            Self::EnergyBoost => {
                player.max_energy += (player.max_energy / 2).max(1);
                true
            }
            Self::HealthBoost => {
                player.max_health += (player.max_health / 2).max(1);
                true
            }
            Self::Lifesteal => {
                player.upgrades.lifesteal = true;
                true
            }
            Self::BonusNoWaste => {
                player.give_money(player.get_money());
                true
            }
            Self::BonusNoDamage => {
                player.max_health += (player.max_health / 2).max(1);
                player.health = player.max_health;
                true
            }
            Self::BonusKillAll => {
                if !player.upgrades.map
                    && player.limbs.count_mage_eyes() == 0
                    && player.perception < 30
                {
                    player.perception += 10;
                } else if player.detect_mod > -5 {
                    player.detect_mod -= 1;
                } else if player.effects.unlucky.is_active() {
                    player.effects.unlucky.remove();
                } else if player.effects.doomed.is_active() && random().is_multiple_of(7) {
                    player.effects.doomed.remove();
                } else {
                    player.give_money((player.get_money() / 2).max(1));
                }
                true
            }
            Self::BonusNoEnergy => {
                player.max_energy += (player.max_energy / 2).max(1);
                true
            }
            Self::LimbMageEye => {
                if let Some(eye) = player.limbs.pick_eye() {
                    // If we don't have an eye there, then we don't need to pull it out
                    let damage = !eye.is_none();
                    *eye = crate::limbs::Eye::Mage;
                    if damage {
                        crate::limbs::Eye::deal_pickup_damage(player);
                    }
                    true
                } else {
                    false
                }
            }
            Self::LimbSeerEye => {
                if let Some(eye) = player.limbs.pick_eye() {
                    let damage = !eye.is_none();
                    *eye = crate::limbs::Eye::Seer;
                    if damage {
                        crate::limbs::Eye::deal_pickup_damage(player);
                    }
                    true
                } else {
                    false
                }
            }
            Self::FullEnergyDing => {
                player.upgrades.full_energy_ding = true;
                true
            }
            Self::Spell(spell) => {
                let mut stdout = std::io::stdout().lock();
                Player::clear_right_column(&mut stdout);
                player.draw_spells(&mut stdout);
                crate::set_feedback("Pick the slot for your new spell, c to cancel".to_string());
                crate::draw_feedback();
                let index = loop {
                    let mut buf = [0];
                    std::io::stdin().lock().read_exact(&mut buf).unwrap();
                    break match buf[0] {
                        b'1' => 0,
                        b'2' => 1,
                        b'3' => 2,
                        b'4' => 3,
                        b'5' => 4,
                        b'6' => 5,
                        b'c' => return false,
                        _ => {
                            crate::bell(None);
                            continue;
                        }
                    };
                };
                crate::log!("Settings spell slot {index} to {spell}");
                player.known_spells[index] = Some(spell);
                true
            }
        }
    }
    pub fn can_pickup(self, player: &Player) -> bool {
        match self {
            Self::Map => !player.upgrades.map,
            Self::SoftShoes => !player.upgrades.soft_shoes,
            Self::PreciseConvert => !player.upgrades.precise_convert,
            Self::Lifesteal => !player.upgrades.lifesteal,
            Self::FullEnergyDing => !player.upgrades.full_energy_ding,
            _ => true,
        }
    }
    pub fn get_desc(self) -> String {
        match self {
            // Normal upgrades
            Self::Map => get(9),
            Self::SoftShoes => get(10),
            Self::PreciseConvert => crate::debug_only!(get(11)),
            Self::HealthBoost => crate::debug_only!(get(12)),
            Self::EnergyBoost => crate::debug_only!(get(13)),
            Self::Lifesteal => crate::debug_only!(get(14)),
            // Bonuses
            Self::BonusNoWaste => crate::debug_only!(get(15)),
            Self::BonusNoDamage => get(16),
            Self::BonusKillAll => get(17),
            Self::BonusNoEnergy => crate::debug_only!(get(18)),
            // Save pint
            Self::SavePint => get(19),
            // Limbs
            Self::LimbMageEye => get(20),
            Self::LimbSeerEye => get(21),
            Self::FullEnergyDing => get(22),
            Self::Spell(spell) => spell.get_name().to_string(),
        }
    }
    // For the normal upgrades, can they be meaningfully picked up multiple times?
    // None is everything that is not in the normal upgrade pool
    pub fn is_repeatable(self) -> Option<bool> {
        match self {
            Self::Map
            | Self::SoftShoes
            | Self::PreciseConvert
            | Self::FullEnergyDing
            | Self::Lifesteal => Some(false),
            Self::EnergyBoost | Self::HealthBoost | Self::LimbMageEye | Self::LimbSeerEye => {
                Some(true)
            }
            _ => None,
        }
    }
}
impl std::fmt::Display for UpgradeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Map => write!(f, "map"),
            Self::SoftShoes => write!(f, "soft shoes"),
            Self::SavePint => write!(f, "save pint"),
            Self::PreciseConvert => write!(f, "precise convert"),
            Self::EnergyBoost => write!(f, "energy boost"),
            Self::HealthBoost => write!(f, "health boost"),
            Self::Lifesteal => write!(f, "lifesteal"),
            Self::BonusNoWaste => write!(f, "bonus no waste"),
            Self::BonusNoDamage => write!(f, "bonus no damage"),
            Self::BonusKillAll => write!(f, "bonus kill all"),
            Self::BonusNoEnergy => write!(f, "bonus no energy"),
            Self::LimbMageEye => write!(f, "limb mage eye"),
            Self::LimbSeerEye => write!(f, "limb seer eye"),
            Self::FullEnergyDing => write!(f, "full energy ding"),
            Self::Spell(spell) => write!(f, "Spell ({spell})"),
        }
    }
}
impl std::str::FromStr for UpgradeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut args = s.trim().split(' ');
        match args.next() {
            Some(arg) => match arg {
                "map" => Ok(Self::Map),
                "soft_shoes" => Ok(Self::SoftShoes),
                "save_pint" => Ok(Self::SavePint),
                "precise_convert" => Ok(Self::PreciseConvert),
                "energy_boost" => Ok(Self::EnergyBoost),
                "health_boost" => Ok(Self::HealthBoost),
                "lifesteal" => Ok(Self::Lifesteal),
                "bonus_no_waste" => Ok(Self::BonusNoWaste),
                "bonus_no_damage" => Ok(Self::BonusNoDamage),
                "bonus_kill_all" => Ok(Self::BonusKillAll),
                "bonus_no_energy" => Ok(Self::BonusNoEnergy),
                "limb_mage_eye" => Ok(Self::LimbMageEye),
                "limb_seer_eye" => Ok(Self::LimbSeerEye),
                "full_energy_ding" => Ok(Self::FullEnergyDing),
                other => Err(format!("{other} is not a valid upgrade")),
            },
            None => Err("Need to specify what upgrade".to_string()),
        }
    }
}
impl Random for UpgradeType {
    fn random() -> Self {
        // save pint and bonuses intentionally ommitted
        match random() % 9 {
            0 => Self::Map,
            1 => Self::SoftShoes,
            2 => Self::PreciseConvert,
            3 => Self::EnergyBoost,
            4 => Self::HealthBoost,
            5 => Self::Lifesteal,
            6 => Self::LimbMageEye,
            7 => Self::LimbSeerEye,
            8 => Self::FullEnergyDing,
            _ => unreachable!("Le fucked is up"),
        }
    }
}
impl FromBinary for UpgradeType {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Self::Map,
            1 => Self::SoftShoes,
            2 => Self::SavePint,
            3 => Self::PreciseConvert,
            4 => Self::EnergyBoost,
            5 => Self::HealthBoost,
            6 => Self::Lifesteal,
            7 => Self::BonusNoWaste,
            8 => Self::BonusNoDamage,
            9 => Self::BonusKillAll,
            10 => Self::BonusNoEnergy,
            11 => Self::LimbMageEye,
            12 => Self::LimbSeerEye,
            13 => Self::FullEnergyDing,
            14 => Self::Spell(crate::spell::Spell::from_binary(binary)?),
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not get UpgradeType from binary",
                ));
            }
        })
    }
}
impl ToBinary for UpgradeType {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        match self {
            Self::Map => 0_u8.to_binary(binary),
            Self::SoftShoes => 1_u8.to_binary(binary),
            Self::SavePint => 2_u8.to_binary(binary),
            Self::PreciseConvert => 3_u8.to_binary(binary),
            Self::EnergyBoost => 4_u8.to_binary(binary),
            Self::HealthBoost => 5_u8.to_binary(binary),
            Self::Lifesteal => 6_u8.to_binary(binary),
            Self::BonusNoWaste => 7_u8.to_binary(binary),
            Self::BonusNoDamage => 8_u8.to_binary(binary),
            Self::BonusKillAll => 9_u8.to_binary(binary),
            Self::BonusNoEnergy => 10_u8.to_binary(binary),
            Self::LimbMageEye => 11_u8.to_binary(binary),
            Self::LimbSeerEye => 12_u8.to_binary(binary),
            Self::FullEnergyDing => 13_u8.to_binary(binary),
            Self::Spell(spell) => {
                14_u8.to_binary(binary)?;
                spell.to_binary(binary)
            }
        }
    }
}
