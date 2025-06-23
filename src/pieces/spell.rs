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
    pub fn on_step(self, stepper: Stepper) {
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
                let health = &mut enemy.try_write().unwrap().health;
                if *health < 3 {
                    *health = 1;
                } else {
                    *health -= 2;
                }
            }
        }
        if let Some(caster) = self.caster.upgrade() {
            caster.try_write().unwrap().health += 2;
        }
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
