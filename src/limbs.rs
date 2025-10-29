use crate::{FromBinary, ToBinary};
use std::io::Read;

pub const LIMB_LOSS_HEALTH_WEIGHT: f64 = 5.0;
pub const LIMB_LOSS_ENERGY_WEIGHT: f64 = 5.0;
pub const LIMB_LOSS_DAMAGE_WEIGHT: f64 = 15.0;

#[derive(Debug, Clone, Copy)]
pub struct Limbs {
    left_eye: Eye,
    right_eye: Eye,
}
macro_rules! eye {
    ($name: ident, $method: ident) => {
        pub fn $name(&self) -> usize {
            let mut out = 0;
            if self.left_eye.$method() {
                out += 1
            }
            if self.right_eye.$method() {
                out += 1
            }
            out
        }
    };
}
// General purpose
impl Limbs {
    pub fn new() -> Limbs {
        Limbs {
            left_eye: {
                if crate::SETTINGS.difficulty() >= crate::Difficulty::Hard {
                    Eye::Seer
                } else {
                    Eye::new()
                }
            },
            right_eye: {
                if crate::SETTINGS.difficulty() >= crate::Difficulty::Hard {
                    Eye::None
                } else {
                    Eye::new()
                }
            },
        }
    }
    pub fn set(&mut self, slot: String, choice: String) -> Result<(), String> {
        match slot.trim() {
            "left_eye" => self.left_eye = choice.parse()?,
            "right_eye" => self.right_eye = choice.parse()?,
            other => return Err(format!("{other} is not a valid limb")),
        }
        Ok(())
    }
    pub fn remove_random_limb(&mut self) {
        *crate::feedback() = format!(
            "{}You lost your {}.\x1b[0m",
            crate::Style::new().red().intense(true),
            match crate::random() % 2 {
                0 => {
                    self.left_eye.remove();
                    "left eye"
                }
                1 => {
                    self.right_eye.remove();
                    "right eye"
                }
                _ => unreachable!("Zoinks, Scoob! It's the Gay Blade!"),
            }
        );
        crate::bell(Some(&mut std::io::stdout()));
    }
    pub fn pick_eye(&mut self) -> Option<&mut Eye> {
        crate::set_feedback("L for left eye, R for right eye, C for cancel".to_string());
        crate::draw_feedback();
        let mut buf = [0];
        let mut lock = std::io::stdin().lock();
        loop {
            lock.read_exact(&mut buf).unwrap();
            buf[0].make_ascii_uppercase();
            match buf[0] {
                b'L' | b'1' => break Some(&mut self.left_eye),
                b'R' | b'2' => break Some(&mut self.right_eye),
                _ => {}
            }
        }
    }
    pub fn draw(&self, mut start: crate::Vector, lock: &mut impl std::io::Write) {
        crossterm::queue!(lock, start.to_move()).unwrap();
        write!(lock, "left eye: ").unwrap();
        self.left_eye.get_name(lock);

        start.down_mut();
        crossterm::queue!(lock, start.to_move()).unwrap();
        write!(lock, "right eye: ").unwrap();
        self.right_eye.get_name(lock);
    }
}
impl FromBinary for Limbs {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Limbs {
            left_eye: Eye::from_binary(binary)?,
            right_eye: Eye::from_binary(binary)?,
        })
    }
}
impl ToBinary for Limbs {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        self.left_eye.to_binary(binary)?;
        self.right_eye.to_binary(binary)
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Eye {
    // No eye, if you only have 1 eye, then your base will be halved, if you have no eyes, you
    // can't see
    None,
    // 1: More perception
    // 2: Even more perception
    Normal,
    // 1: Can tell where enemies are(can't see windup/type)
    // 2: Can see enemies clearly
    Mage,
    // 1: see when a ranged attack has been decided to happen
    //  (have background of player change?)
    // 2: see where they are aiming
    Seer,
}
impl Eye {
    pub const fn new() -> Eye {
        Eye::Normal
    }
    pub const fn is_normal(self) -> bool {
        matches!(self, Eye::Normal)
    }
    pub const fn is_mage(self) -> bool {
        matches!(self, Eye::Mage)
    }
    pub const fn is_seer(self) -> bool {
        matches!(self, Eye::Seer)
    }
    pub const fn is_none(self) -> bool {
        matches!(self, Eye::None)
    }
    pub const fn remove(&mut self) {
        *self = Eye::None;
    }
    pub fn deal_pickup_damage(player: &mut crate::Player) {
        // + 4 for doomed
        // + 2 for unlucky
        // (non stacking)
        let mut damage = crate::enemy::luck_roll8(player) as usize + 1;
        // If the player is below 40% energy, they are "tired" and therefore "clumsy"
        // meaning that the damage rolls shift from 1 2 3 4 5 6 7 8 to 3 4 5 6 7 8 9 10
        // If they are additionally doomed then the damage rolls shift to 7 8 9 10 10 10 10 10
        if (player.energy * 10) / player.max_energy < 4 {
            damage += 2;
        }
        let _ = player.attacked(damage, "stupidity".to_string(), None);
    }
    pub fn get_name(&self, lock: &mut impl std::io::Write) {
        match self {
            Eye::None => return,
            Eye::Normal => write!(lock, "normal"),
            Eye::Mage => write!(lock, "mage"),
            Eye::Seer => write!(lock, "seer"),
        }
        .unwrap()
    }
}
impl Limbs {
    eye!(count_normal_eyes, is_normal);
    eye!(count_mage_eyes, is_mage);
    eye!(count_seer_eyes, is_seer);
    pub fn count_eyes(&self) -> usize {
        let mut count = 0;
        if !self.left_eye.is_none() {
            count += 1;
        }
        if !self.right_eye.is_none() {
            count += 1;
        }
        count
    }
}
impl std::str::FromStr for Eye {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim() {
            "none" => Eye::None,
            "normal" => Eye::Normal,
            "mage" => Eye::Mage,
            "seer" => Eye::Seer,
            other => return Err(format!("{other} is not a valid eye")),
        })
    }
}
impl FromBinary for Eye {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Eye::None,
            1 => Eye::Normal,
            2 => Eye::Mage,
            3 => Eye::Seer,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "My name is Professor Bug",
                ));
            }
        })
    }
}
impl ToBinary for Eye {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        match self {
            Eye::None => 0_u8,
            Eye::Normal => 1,
            Eye::Mage => 2,
            Eye::Seer => 3,
        }
        .to_binary(binary)
    }
}
