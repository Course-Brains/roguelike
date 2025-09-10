use crate::{FromBinary, Player, Random, Style, ToBinary, player::Duration, random};
use std::io::Read;

const AVAILABLE: Style = *Style::new().green();
const UNAVAILABLE: Style = *Style::new().red();

#[derive(Clone, Copy, Debug)]
pub struct Upgrades {
    pub map: bool,
    pub soft_shoes: bool,
    pub precise_convert: bool,
    pub lifesteal: bool,
}
impl Upgrades {
    pub const fn new() -> Upgrades {
        Upgrades {
            map: false,
            soft_shoes: false,
            precise_convert: false,
            lifesteal: false,
        }
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
        })
    }
}
impl ToBinary for Upgrades {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        self.map.to_binary(binary)?;
        self.soft_shoes.to_binary(binary)?;
        self.precise_convert.to_binary(binary)?;
        self.lifesteal.to_binary(binary)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum UpgradeType {
    Map,
    SoftShoes,
    SavePint, // no corresponding upgrade field
    PreciseConvert,
    EnergyBoost,   // max energy (exponential cost?)
    HealthBoost,   // max health ...
    Lifesteal,     // health on kill
    BonusNoWaste,  // double money
    BonusNoDamage, // +50% max health & full heal
    BonusKillAll,  // complicated, look at on_pickup
    BonusNoEnergy, // +50% max energy
    LimbMageEye,
    LimbSeerEye,
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
                    && player.perception < 50
                {
                    player.perception += 10;
                } else if player.detect_mod > -5 {
                    player.detect_mod -= 1;
                } else if player.effects.unlucky.is_active() {
                    player.effects.unlucky.remove();
                } else if player.effects.doomed.is_active() && random() % 7 == 0 {
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
        }
    }
    pub fn can_pickup(self, player: &Player) -> bool {
        match self {
            Self::Map => !player.upgrades.map,
            Self::SoftShoes => !player.upgrades.soft_shoes,
            Self::PreciseConvert => !player.upgrades.precise_convert,
            Self::Lifesteal => !player.upgrades.lifesteal,
            _ => true,
        }
    }
    pub fn get_desc(self) -> &'static str {
        match self {
            // Normal upgrades
            Self::Map => "A map",
            Self::SoftShoes => "A pair of particularly soft shoes",
            Self::PreciseConvert => "Clean needles",
            Self::HealthBoost => "Additional flesh",
            Self::EnergyBoost => "An adrenal gland",
            Self::Lifesteal => "A butcher's knife",
            // Bonuses
            Self::BonusNoWaste => "A result of your greed",
            Self::BonusNoDamage => "A result of your fear",
            Self::BonusKillAll => "A result of your cruelty",
            Self::BonusNoEnergy => "A result of your ",
            // Save pint
            Self::SavePint => "A savepint",
            // Limbs
            Self::LimbMageEye => "A mage's eye",
            Self::LimbSeerEye => "A seer's eye",
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
        }
    }
}
impl std::str::FromStr for UpgradeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
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
            other => Err(format!("{other} is not a valid upgrade")),
        }
    }
}
impl Random for UpgradeType {
    fn random() -> Self {
        // save pint and bonuses intentionally ommitted
        match random() % 8 {
            0 => Self::Map,
            1 => Self::SoftShoes,
            2 => Self::PreciseConvert,
            3 => Self::EnergyBoost,
            4 => Self::HealthBoost,
            5 => Self::Lifesteal,
            6 => Self::LimbMageEye,
            7 => Self::LimbSeerEye,
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
            Self::Map => 0_u8,
            Self::SoftShoes => 1,
            Self::SavePint => 2,
            Self::PreciseConvert => 3,
            Self::EnergyBoost => 4,
            Self::HealthBoost => 5,
            Self::Lifesteal => 6,
            Self::BonusNoWaste => 7,
            Self::BonusNoDamage => 8,
            Self::BonusKillAll => 9,
            Self::BonusNoEnergy => 10,
            Self::LimbMageEye => 11,
            Self::LimbSeerEye => 12,
        }
        .to_binary(binary)
    }
}
