#[derive(Clone, Copy, Debug)]
pub enum ItemType {
    MageSight,
}
impl ItemType {
    // What is listed in the inventory
    pub fn name(self, out: &mut impl std::io::Write) {
        match self {
            Self::MageSight => write!(out, "Scroll of magical sight").unwrap(),
        }
    }
    // What happens when it is used
    // returns whether or not it succeeded and should take the turn
    pub fn enact(self, state: &mut crate::State) -> bool {
        match self {
            Self::MageSight => todo!(),
        }
    }
    // The price to pick up
    pub fn price(self) -> usize {
        match self {
            Self::MageSight => 5,
        }
    }
    // What is said when on the ground
    pub fn get_desc(self) -> &'static str {
        match self {
            Self::MageSight => "Scroll of magical sight",
        }
    }
}
impl std::fmt::Display for ItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::MageSight => write!(f, "mage sight"),
        }
    }
}
