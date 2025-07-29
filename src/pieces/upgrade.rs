use crate::{Entity, FromBinary, Random, ToBinary, upgrades::UpgradeType};
#[derive(Clone, Copy, Debug)]
pub struct Upgrade(UpgradeType);
impl Upgrade {
    pub fn new(which: Option<UpgradeType>) -> Upgrade {
        Upgrade(which.unwrap_or(UpgradeType::random()))
    }
    pub fn render(&self, player: &crate::Player) -> (char, Option<crate::Style>) {
        self.0.render(player)
    }
    pub fn on_step(&self, stepper: Entity<'_>) -> bool {
        if let Entity::Player(player) = stepper {
            if player.have_money(self.0.cost()) && self.0.can_pickup(player) {
                return self.0.on_pickup(player);
            }
        }
        false
    }
    pub fn get_desc(&self, out: &mut impl std::io::Write) {
        write!(
            out,
            "Upgrade: {}{}\x1b[0m, cost {}{}\x1b[0m",
            crate::Style::new().green(),
            self.0.get_desc(),
            crate::Style::new().red(),
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
impl FromBinary for Upgrade {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Upgrade(UpgradeType::from_binary(binary)?))
    }
}
impl ToBinary for Upgrade {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        self.0.to_binary(binary)
    }
}
