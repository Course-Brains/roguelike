#[derive(Clone, Copy, Debug)]
pub enum ItemType {
    Testing,
}
impl ItemType {
    pub fn name(self, out: &mut impl std::io::Write) {
        match self {
            Self::Testing => write!(out, "Test").unwrap(),
        }
    }
    // returns whether or not it succeeded and should take the turn
    pub fn enact(self, state: &mut crate::State) -> bool {
        match self {
            Self::Testing => todo!(),
        }
    }
    pub fn price(self) -> usize {
        match self {
            Self::Testing => 1,
        }
    }
    pub fn get_desc(self) -> &'static str {
        match self {
            Self::Testing => "A test",
        }
    }
}
impl std::fmt::Display for ItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Testing => write!(f, "Testing"),
        }
    }
}
