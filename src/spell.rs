use crate::{
    Board, Enemy, Entity, FromBinary, Player, Style, ToBinary, Vector, board::Special, ray_cast,
};
use std::io::{Read, Write};
use std::sync::{Arc, RwLock};
pub struct SpellCircle {
    pub spell: Spell,
    pub pos: Vector,
    // None is player
    pub caster: Option<Arc<RwLock<Enemy>>>,
    // Needed for normal spells, not contact
    pub aim: Option<Vector>,
}
impl SpellCircle {
    // returns true if the circle should be kept (false = removal)
    pub fn update(&self, board: &mut Board, player: &mut Player) -> bool {
        match &self.spell {
            Spell::Normal(spell) => {
                spell.cast(self.caster.clone(), player, board, Some(self.pos), self.aim);
                true
            }
            Spell::Contact(spell) => {
                if let Some(enemy) = board.get_enemy(self.pos, None) {
                    spell.cast(enemy.into(), Entity::new(self.caster.clone(), player));
                    return false;
                }
                true
            }
        }
    }
}
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Spell {
    Contact(ContactSpell),
    Normal(NormalSpell),
}
impl Spell {
    pub fn unwrap_contact(&self) -> &ContactSpell {
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
            None => Err("You must specify the spell".to_string()),
        }
    }
}
impl FromBinary for Spell {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match bool::from_binary(binary)? {
            true => Spell::Contact(ContactSpell::from_binary(binary)?),
            false => Spell::Normal(NormalSpell::from_binary(binary)?),
        })
    }
}
impl ToBinary for Spell {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        match self {
            Self::Contact(spell) => {
                true.to_binary(binary)?;
                spell.to_binary(binary)
            }
            Self::Normal(spell) => {
                false.to_binary(binary)?;
                spell.to_binary(binary)
            }
        }
    }
}
// requires you to be able to touch the target(aka within the 3 by 3 around you) or in the case of
// spell circles, requires them to stand on it, then it triggers
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ContactSpell {
    // Take health from the target
    DrainHealth,
}
impl ContactSpell {
    pub fn cast(&self, target: Entity<'_>, caster: Entity<'_>) {
        if let Entity::Player(_) = caster {
            crate::stats().add_spell(Spell::Contact(*self));
        }
        match self {
            Self::DrainHealth => {
                match target {
                    Entity::Player(player) => {
                        // can't handle if it is a player because then we would have 2 &mut to the
                        // player which can't happen
                        let binding = caster.unwrap_enemy();
                        let mut caster = binding.try_write().unwrap();
                        let damage = (crate::enemy::luck_roll8(player) as usize / 2) + 1;
                        let _ = player.attacked(damage * 5, caster.variant.kill_name());
                        caster.health += damage;
                    }
                    Entity::Enemy(target) => {
                        if let Entity::Enemy(caster) = &caster {
                            if Arc::ptr_eq(caster, &target) {
                                return;
                            }
                        }
                        let damage = (crate::random() as usize & 3) + 1;
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
impl FromBinary for ContactSpell {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Self::DrainHealth,
            _ => unreachable!("Failed to get ContactSpell from binary"),
        })
    }
}
impl ToBinary for ContactSpell {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        match self {
            Self::DrainHealth => 0_u8,
        }
        .to_binary(binary)
    }
}
// Cast by you normally, might have their own activation conditions, in the case of spell circles,
// they will cast repeatedly until they don't have enough mana
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum NormalSpell {
    // swap position with a random enemy within a 30 radius square
    Swap,
    // A fireball, has AOE
    BidenBlast,
    // Get information about the target(eg. health)
    Identify,
}
impl NormalSpell {
    // aim is position not direction
    // origin of None means to get it from the caster(including the player)
    pub fn cast(
        &self,
        caster: Option<Arc<RwLock<Enemy>>>,
        player: &mut Player,
        board: &mut Board,
        origin: Option<Vector>,
        aim: Option<Vector>,
    ) -> bool {
        if caster.is_none() {
            crate::stats().add_spell(Spell::Normal(*self));
        }
        match self {
            Self::Swap => {
                let addr = caster.as_ref().map(|arc| Arc::as_ptr(arc).addr());
                if let Some(swap) = board.pick_near(addr, get_pos(&caster, player), 30) {
                    let swap = swap.upgrade().unwrap();
                    if is_within_flood(&caster) != swap.try_read().unwrap().reachable {
                        // One is within the flood and one isn't so we need to reflood
                        crate::RE_FLOOD.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    match caster {
                        Some(caster) => {
                            std::mem::swap(
                                &mut swap.try_write().unwrap().pos,
                                &mut caster.try_write().unwrap().pos,
                            );
                        }
                        None => std::mem::swap(&mut swap.try_write().unwrap().pos, &mut player.pos),
                    }
                    return true;
                }
                false
            }
            Self::BidenBlast => {
                let aim = aim.unwrap();
                let origin = origin.unwrap_or(get_pos(&caster, player));
                let path = ray_cast(origin, aim, board, None, caster.is_none(), player.pos).0;
                let last_pos = *path.last().unwrap();
                let render_bounds = board.get_render_bounds(player);

                // drawing projectile
                for pos in path.iter() {
                    let special = board.add_special(fireball(*pos));
                    if board.is_visible(*pos, render_bounds.clone()) {
                        board.smart_render(player);
                        crate::proj_delay();
                    }
                    std::mem::drop(special);
                }
                // drawing explosion
                if board.is_visible(last_pos, render_bounds) {
                    let mut specials = Vec::new();
                    specials.push(board.add_special(explosion(last_pos)));
                    board.smart_render(player);
                    std::thread::sleep(crate::PROJ_DELAY * 4);
                    for pos in last_pos.list_near(2).iter() {
                        specials.push(board.add_special(explosion(*pos)));
                    }
                    board.smart_render(player);
                    std::thread::sleep(crate::PROJ_DELAY * 4);
                    std::mem::drop(specials);
                    board.smart_render(player);
                }
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
            Self::Identify => {
                assert!(caster.is_none(), "Identify can only be cast by the player");
                let aim = aim.unwrap();
                crossterm::queue!(
                    std::io::stdout(),
                    crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
                )
                .unwrap();
                println!("At position: {aim}");
                if let Some(piece) = &board[aim] {
                    println!("  There is a piece: {piece}");
                }
                if let Some(enemy) = board.get_enemy(aim, None) {
                    let enemy = enemy.try_read().unwrap();
                    println!(
                        "  There is an enemy: {} with {} health",
                        enemy.variant.kill_name(),
                        enemy.health
                    );
                    if !enemy.reward {
                        println!("    It seems to be missing something");
                    }
                }
                if player.pos == aim {
                    println!("  The player is there:");
                    if player.effects.has_none() {
                        println!("    and has no effects");
                    } else {
                        player.effects.list();
                    }
                }
                true
            }
        }
    }
    pub fn cast_time(&self) -> usize {
        match self {
            Self::Swap => 3,
            Self::BidenBlast => 4,
            Self::Identify => 0,
        }
    }
}
impl std::str::FromStr for NormalSpell {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "swap" => Ok(Self::Swap),
            "biden_blast" => Ok(Self::BidenBlast),
            "identify" => Ok(Self::Identify),
            other => Err(format!("\"{other}\" is not a valid normal spell")),
        }
    }
}
impl FromBinary for NormalSpell {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Self::Swap,
            1 => Self::BidenBlast,
            2 => Self::Identify,
            _ => unreachable!("Failed to get NormalSpell from binary"),
        })
    }
}
impl ToBinary for NormalSpell {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        match self {
            Self::Swap => 0_u8,
            Self::BidenBlast => 1,
            Self::Identify => 2,
        }
        .to_binary(binary)
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
