use crate::{FromBinary, Player, Random, Style, ToBinary, player::Duration, random};
use std::io::Read;

const SYMBOL: char = 'U';
const AVAILABLE: Style = *Style::new().green();
const UNAVAILABLE: Style = *Style::new().red();

#[derive(Clone, Copy, Debug)]
pub struct Upgrades {
    pub mage_eye: usize,
    pub map: bool,
    pub soft_shoes: bool,
    pub precise_convert: bool,
}
impl Upgrades {
    pub const fn new() -> Upgrades {
        Upgrades {
            mage_eye: 0,
            map: false,
            soft_shoes: false,
            precise_convert: false,
        }
    }
}
impl FromBinary for Upgrades {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Upgrades {
            mage_eye: usize::from_binary(binary)?,
            map: bool::from_binary(binary)?,
            soft_shoes: bool::from_binary(binary)?,
            precise_convert: bool::from_binary(binary)?,
        })
    }
}
impl ToBinary for Upgrades {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        self.mage_eye.to_binary(binary)?;
        self.map.to_binary(binary)?;
        self.soft_shoes.to_binary(binary)?;
        self.precise_convert.to_binary(binary)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum UpgradeType {
    MageEye,
    Map,
    SoftShoes,
    SavePint, // no corresponding upgrade field
    PreciseConvert,
}
impl UpgradeType {
    pub fn render(&self, player: &Player) -> (char, Option<Style>) {
        (
            SYMBOL,
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
            Self::MageEye => 200,
            Self::Map => 300,
            Self::SoftShoes => 200,
            Self::SavePint => 0,
            Self::PreciseConvert => 150,
        }
    }
    pub fn on_pickup(self, player: &mut Player) -> bool {
        match self {
            Self::MageEye => {
                player.effects.mage_sight = Duration::Infinite;
                player.upgrades.mage_eye += 1;
                let _ = player.attacked(
                    ((crate::enemy::luck_roll8(player) as usize / 2) + 1) * 5,
                    "stupidity",
                );
                if player.upgrades.mage_eye == 2 && player.effects.unlucky.is_active() {
                    player.inspect = false;
                    crate::set_desc("You feel whole");
                    player.max_energy += 1;
                    player.effects.unlucky.remove();
                }
                true
            }
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
        }
    }
    pub fn can_pickup(self, player: &Player) -> bool {
        match self {
            Self::MageEye => player.upgrades.mage_eye < 2,
            Self::Map => !player.upgrades.map,
            Self::SoftShoes => !player.upgrades.soft_shoes,
            Self::PreciseConvert => !player.upgrades.precise_convert,
            _ => true,
        }
    }
    pub fn get_desc(self) -> &'static str {
        match self {
            Self::MageEye => "A mage's eye",
            Self::Map => "A map",
            Self::SoftShoes => "A pair of particularly soft shoes",
            Self::SavePint => "A savepint",
            Self::PreciseConvert => "Clean needles",
        }
    }
}
impl std::fmt::Display for UpgradeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MageEye => write!(f, "mage eye"),
            Self::Map => write!(f, "map"),
            Self::SoftShoes => write!(f, "soft shoes"),
            Self::SavePint => write!(f, "save pint"),
            Self::PreciseConvert => write!(f, "precise convert"),
        }
    }
}
impl std::str::FromStr for UpgradeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "mage_eye" => Ok(Self::MageEye),
            "map" => Ok(Self::Map),
            "soft_shoes" => Ok(Self::SoftShoes),
            "save_pint" => Ok(Self::SavePint),
            "precise_convert" => Ok(Self::PreciseConvert),
            other => Err(format!("{other} is not a valid upgrade")),
        }
    }
}
impl Random for UpgradeType {
    fn random() -> Self {
        match random() % 4 {
            0 => Self::MageEye,
            1 => Self::Map,
            2 => Self::SoftShoes,
            3 => Self::PreciseConvert,
            // Save pint cannot be made from random calls
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
            0 => Self::MageEye,
            1 => Self::Map,
            2 => Self::SoftShoes,
            3 => Self::SavePint,
            4 => Self::PreciseConvert,
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
            Self::MageEye => 0_u8,
            Self::Map => 1,
            Self::SoftShoes => 2,
            Self::SavePint => 3,
            Self::PreciseConvert => 4,
        }
        .to_binary(binary)
    }
}
