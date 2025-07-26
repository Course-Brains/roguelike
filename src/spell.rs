use crate::{Board, Enemy, Entity, Player, Style, Vector, board::Special, ray_cast};
use std::sync::{Arc, RwLock};
pub struct SpellCircle {
    pub spell: Spell,
    pub pos: Vector,
    // None is player
    pub caster: Option<Arc<RwLock<Enemy>>>,
}
impl SpellCircle {}
pub enum Spell {
    Contact(ContactSpell),
    Normal(NormalSpell),
}
impl Spell {
    pub fn unwrap_contact<'a>(&'a self) -> &'a ContactSpell {
        match self {
            Spell::Contact(contact) => contact,
            Spell::Normal(_) => panic!("Called unwrap_contact on a normal spell"),
        }
    }
    /*pub fn unwrap_normal<'a>(&'a self) -> &'a NormalSpell {
        match self {
            Self::Contact(_) => panic!("Called unwrap_normal on a contact spell"),
            Self::Normal(normal) => normal,
        }
    }
    pub fn cast_time(&self) -> usize {
        match self {
            Spell::Contact(spell) => spell.cast_time(),
            Spell::Normal(spell) => spell.cast_time(),
        }
    }*/
}
impl std::str::FromStr for Spell {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(' ');
        match split.next() {
            Some(spell_type) => {
                let args: String = split.map(|x| x.to_string() + " ").collect();
                match spell_type {
                    "contact" => Ok(Spell::Contact(args.parse()?)),
                    "normal" => Ok(Spell::Normal(args.parse()?)),
                    other => Err(format!("\"{other}\" is not a valid spell type")),
                }
            }
            None => Err(format!("You must specify the spell")),
        }
    }
}
// requires you to be able to touch the target(aka within the 3 by 3 around you) or in the case of
// spell circles, requires them to stand on it, then it triggers
pub enum ContactSpell {
    // Take health from the target
    DrainHealth,
}
impl ContactSpell {
    pub fn cast(&self, target: Entity<'_>, caster: Entity<'_>) {
        match self {
            Self::DrainHealth => {
                match target {
                    Entity::Player(player) => {
                        // can't handle if it is a player because then we would have 2 &mut to the
                        // player which can't happen
                        let binding = caster.unwrap_enemy();
                        let mut caster = binding.try_write().unwrap();
                        let damage = crate::enemy::luck_roll8(player) as usize / 2;
                        let _ = player.attacked(damage * 5, caster.variant.kill_name());
                        caster.health += damage;
                    }
                    Entity::Enemy(target) => {
                        if let Entity::Enemy(caster) = &caster {
                            if Arc::ptr_eq(caster, &target) {
                                return;
                            }
                        }
                        let damage = (crate::random() as usize * 3) + 1;
                        target.try_write().unwrap().attacked(damage);
                        match caster {
                            Entity::Player(player) => player.heal(damage * 5),
                            Entity::Enemy(enemy) => enemy.try_write().unwrap().health += damage,
                        }
                    }
                }
            }
        }
    }
    pub fn cast_time(&self) -> usize {
        match self {
            Self::DrainHealth => 3,
        }
    }
}
impl std::str::FromStr for ContactSpell {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "drain_health" => Ok(Self::DrainHealth),
            other => Err(format!("\"{other}\" is not a contact spell")),
        }
    }
}
// Cast by you normally, might have their own activation conditions, in the case of spell circles,
// they will cast repeatedly until they don't have enough mana
pub enum NormalSpell {
    // swap position with a random enemy within a 30 radius square
    Swap,
    // A fireball, has AOE
    BidenBlast,
}
impl NormalSpell {
    // aim is position not direction
    pub fn cast(
        &self,
        caster: Option<Arc<RwLock<Enemy>>>,
        player: &mut Player,
        board: &mut Board,
        aim: Option<Vector>,
    ) -> bool {
        match self {
            Self::Swap => {
                let addr = caster.as_ref().map(|arc| Arc::as_ptr(arc).addr());
                if let Some(swap) = board.pick_near(addr, get_pos(&caster, player), 30) {
                    let swap = swap.upgrade().unwrap();
                    if is_within_flood(&caster) != swap.try_read().unwrap().reachable {
                        // One is within the flood and one isn't so we need to reflood
                        crate::RE_FLOOD.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    let caster_pos = &mut get_pos(&caster, player);
                    let swap_pos = &mut swap.try_write().unwrap().pos;
                    std::mem::swap(caster_pos, swap_pos);
                    return true;
                }
                false
            }
            Self::BidenBlast => {
                let aim = aim.unwrap();
                let path = ray_cast(
                    get_pos(&caster, player),
                    aim,
                    board,
                    None,
                    caster.is_none(),
                    player.pos,
                )
                .0;
                let last_pos = *path.last().unwrap();
                let reset_to = board.specials.len();
                // drawing projectile
                for pos in path.iter() {
                    board.specials.push(fireball(*pos));
                    board.smart_render(player);
                    crate::proj_delay();
                    board.specials.pop();
                }
                // drawing explosion
                board.specials.push(explosion(last_pos));
                board.smart_render(player);
                std::thread::sleep(crate::PROJ_DELAY * 4);
                for pos in last_pos.list_near(2).iter() {
                    board.specials.push(explosion(*pos));
                }
                board.smart_render(player);
                std::thread::sleep(crate::PROJ_DELAY * 4);
                board.specials.truncate(reset_to);
                board.smart_render(player);
                // dealing damage
                let near = board.get_near(None, last_pos, 3);
                for enemy in near.iter() {
                    enemy
                        .upgrade()
                        .unwrap()
                        .try_write()
                        .unwrap()
                        .attacked(crate::random4() as usize);
                }
                if player.pos.is_near(last_pos, 3) {
                    let _ = player.attacked(
                        crate::random4() as usize * 5,
                        match caster {
                            Some(caster) => caster.try_read().unwrap().variant.kill_name(),
                            None => "a lack of depth perception",
                        },
                    );
                }
                true
            }
        }
    }
    pub fn cast_time(&self) -> usize {
        match self {
            Self::Swap => 3,
            Self::BidenBlast => 4,
        }
    }
}
impl std::str::FromStr for NormalSpell {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "swap" => Ok(Self::Swap),
            "biden_blast" => Ok(Self::BidenBlast),
            other => Err(format!("\"{other}\" is not a valid normal spell")),
        }
    }
}
fn fireball(pos: Vector) -> Special {
    Special::new(pos, '●', Some(*Style::new().red().intense(true)))
}
fn explosion(pos: Vector) -> Special {
    Special::new(
        pos,
        ' ',
        Some(*Style::new().background_red().intense_background(true)),
    )
}
fn get_pos(caster: &Option<Arc<RwLock<Enemy>>>, player: &Player) -> Vector {
    match caster {
        Some(arc) => arc.try_read().unwrap().pos,
        None => player.pos,
    }
}
fn is_within_flood(caster: &Option<Arc<RwLock<Enemy>>>) -> bool {
    caster
        .as_ref()
        .is_none_or(|arc| arc.try_read().unwrap().reachable)
}
