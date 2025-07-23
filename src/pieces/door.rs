use crate::{FromBinary, ToBinary};
#[derive(Clone, Copy, Debug)]
pub struct Door {
    pub open: bool,
}
impl Door {
    pub fn render(&self, pos: crate::Vector, board: &crate::Board) -> (char, Option<crate::Style>) {
        match self.open {
            true => (super::wall::QUAD, Some(*crate::Style::new().green())),
            false => (
                super::wall::Wall::render(pos, board),
                Some(*crate::Style::new().red()),
            ),
        }
    }
    pub fn has_collision(&self) -> bool {
        !self.open
    }
    pub fn get_desc(&self, lock: &mut impl std::io::Write) {
        match self.open {
            true => write!(lock, "An open door").unwrap(),
            false => write!(lock, "A closed door").unwrap(),
        }
    }
}
impl std::fmt::Display for Door {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self.open {
            true => write!(f, "Open Door"),
            false => write!(f, "Closed Door"),
        }
    }
}
impl std::str::FromStr for Door {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "open" => Ok(Door { open: true }),
            "closed" => Ok(Door { open: false }),
            invalid => Err(format!("{invalid} is not open or closed")),
        }
    }
}
impl FromBinary for Door {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Door {
            open: bool::from_binary(binary)?,
        })
    }
}
impl ToBinary for Door {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        self.open.to_binary(binary)
    }
}
