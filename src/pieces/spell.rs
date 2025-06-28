use crate::{Enemy, Player, Style, enemy::Variant};
use std::sync::{Arc, RwLock, Weak};
#[derive(Clone, Debug)]
pub struct Spell {
    caster: Weak<RwLock<Enemy>>,
}
impl Spell {
    pub const SYMBOL: char = '∆';
    pub const STYLE: Style = *Style::new().purple().intense(true);
    pub fn new(caster: Weak<RwLock<Enemy>>) -> Spell {
        Spell { caster }
    }
    pub fn on_step(&self, stepper: Stepper) {
        match stepper {
            Stepper::Player(player) => {
                // if you are wondering why it says the mage was teleporting when it killed you,
                // this is why
                let _ = player.attacked(20, Variant::mage().kill_name());
            }
            Stepper::Enemy(enemy) => {
                if Arc::as_ptr(&enemy).addr() == self.caster.as_ptr().addr() {
                    return;
                }
                enemy.try_write().unwrap().attacked(4);
            }
        }
        if let Some(caster) = self.caster.upgrade() {
            caster.try_write().unwrap().health += 2;
        }
    }
}
impl std::fmt::Display for Spell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Spell")
    }
}
pub enum Stepper<'a> {
    Player(&'a mut Player),
    Enemy(Arc<RwLock<Enemy>>),
}
impl<'a> From<&'a mut Player> for Stepper<'a> {
    fn from(value: &'a mut Player) -> Self {
        Stepper::Player(value)
    }
}
impl<'a> From<Arc<RwLock<Enemy>>> for Stepper<'a> {
    fn from(value: Arc<RwLock<Enemy>>) -> Self {
        Stepper::Enemy(value)
    }
}
