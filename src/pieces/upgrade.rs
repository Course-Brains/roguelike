use crate::{Random, pieces::spell::Stepper, upgrades::UpgradeType};
#[derive(Clone, Copy, Debug)]
pub struct Upgrade(UpgradeType);
impl Upgrade {
    pub fn new(which: Option<UpgradeType>) -> Upgrade {
        Upgrade(which.unwrap_or(UpgradeType::random()))
    }
    pub const fn render(&self) -> (char, Option<crate::Style>) {
        self.0.render()
    }
    pub fn on_step(&self, stepper: Stepper<'_>) -> bool {
        if let Stepper::Player(player) = stepper {
            if player.money >= self.0.cost() && self.0.can_pickup(player) {
                self.0.on_pickup(player);
                return true;
            }
        }
        false
    }
    pub fn get_desc(&self, out: &mut impl std::io::Write) {
        write!(
            out,
            "Upgrade: {}{}\x1b[0m, cost {}{}\x1b[0m",
            crate::Style::new().green().enact(),
            self.0.get_desc(),
            crate::Style::new().red().enact(),
            self.0.cost()
        )
        .unwrap()
    }
}
impl std::fmt::Display for Upgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::str::FromStr for Upgrade {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Upgrade(s.parse()?))
    }
}
