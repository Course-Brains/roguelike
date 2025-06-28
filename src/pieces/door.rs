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
    pub fn get_desc(&self) -> &'static str {
        match self.open {
            true => "An open door",
            false => "A closed door",
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
