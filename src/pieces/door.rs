#[derive(Clone, Copy)]
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
}
