use crate::{Board, Direction, Player, Vector, pieces::spell, style::Color};
use std::sync::{Arc, RwLock, RwLockWriteGuard};
const MAGE_RANGE: usize = 30;
const TELE_THRESH: usize = 5;
#[derive(Clone, Copy, Debug)]
pub struct Enemy {
    pub health: usize,
    pub variant: Variant,
    stun: usize,
    windup: usize,
    pub pos: Vector,
    pub active: bool,
    pub reachable: bool,
    pub attacking: bool,
}
impl Enemy {
    pub fn new(pos: Vector, variant: Variant) -> Enemy {
        Enemy {
            health: variant.max_health(),
            variant,
            stun: 0,
            windup: 0,
            pos,
            active: false,
            reachable: false,
            attacking: false,
        }
    }
    pub fn render(&self) -> (char, Option<crate::Style>) {
        (
            match self.variant {
                Variant::Basic | Variant::BasicBoss(_) => '1',
                Variant::Mage(_) | Variant::MageBoss => '2',
            },
            Some({
                let mut out = crate::Style::new();
                if self.variant.is_boss() {
                    out.blue().intense(true);
                } else {
                    if self.active {
                        out.yellow();
                    }
                }
                if self.stun > 0 {
                    out.background_blue();
                } else if self.windup > 0 {
                    out.set_background(self.variant.windup_color())
                        .intense_background(true);
                }
                out
            }),
        )
    }
    pub fn is_stunned(&self) -> bool {
        self.stun > 0
    }
    pub fn is_windup(&self) -> bool {
        self.windup > 0
    }
    pub fn apply_dashstun(&mut self) {
        self.stun += self.variant.dash_stun();
    }
    // returns whether or not it was killed
    pub fn attacked(&mut self) -> bool {
        self.health -= 1;
        self.active = true;
        self.health == 0
    }
    pub fn think(arc: Arc<RwLock<Self>>, addr: usize, board: &mut Board, player: &mut Player) {
        let mut this = arc.try_write().unwrap();
        if !this.active {
            if this.variant.detect(&this, board) {
                this.active = true;
            } else {
                return;
            }
        }
        if this.stun != 0 {
            this.stun -= 1;
            return;
        }
        match this.variant {
            Variant::Basic => {
                if player.pos.x.abs_diff(this.pos.x) < 2 && player.pos.y.abs_diff(this.pos.y) < 2 {
                    this.attacking = true;
                    if this.windup == 0 {
                        this.windup = 3;
                        return;
                    }
                    this.windup -= 1;
                    if this.windup == 0 {
                        if let Err(_) = player.attacked(
                            (crate::random() & 0b0000_0011) as usize + 3,
                            Variant::Basic.kill_name(),
                        ) {
                            this.stun = this.variant.parry_stun();
                        }
                    }
                } else {
                    this.attacking = false;
                    this.windup = 0;
                }
            }
            Variant::Mage(spell) => {
                if this.is_near(player.pos, MAGE_RANGE) {
                    this.attacking = true;
                } else {
                    this.attacking = false;
                    this.windup = 0;
                }
                if this.windup > 1 {
                    this.windup -= 1;
                    return;
                }
                if this.windup == 1 {
                    // cast time BAYBEEE
                    match spell {
                        Spell::Circle(cast_pos) => {
                            if board[cast_pos].is_none() {
                                board[cast_pos] = Some(crate::board::Piece::Spell(
                                    spell::Spell::new(Arc::downgrade(&arc)),
                                ));
                            }
                            this.windup = 0;
                        }
                        Spell::Teleport => {
                            let mut near = Vec::new();
                            for enemy in board.enemies.iter() {
                                if Arc::as_ptr(enemy).addr() == addr {
                                    continue;
                                }
                                let pos = enemy.try_read().unwrap().pos;
                                if pos.x.abs_diff(this.pos.x) < MAGE_RANGE
                                    && pos.y.abs_diff(this.pos.y) < MAGE_RANGE
                                {
                                    near.push(enemy.clone());
                                }
                            }
                            let target = match near.len() > 256 {
                                true => near[crate::random() as usize].clone(),
                                false => near[crate::random() as usize % (near.len() - 1)].clone(),
                            };
                            std::mem::swap(&mut target.try_write().unwrap().pos, &mut this.pos);
                            crate::RE_FLOOD.store(true, std::sync::atomic::Ordering::Relaxed);
                            this.windup = 0;
                        }
                    }
                }
                match crate::random() & 0b0000_0011 {
                    0 => {
                        // teleport
                        if this.is_near(player.pos, TELE_THRESH) {
                            this.windup = 3;
                            this.variant = Variant::Mage(Spell::Teleport);
                        }
                    }
                    1 => {
                        // Alert nearby enemies
                        for enemy in board.enemies.iter_mut() {
                            if Arc::as_ptr(enemy).addr() == addr {
                                continue;
                            }
                            if enemy.try_read().unwrap().is_near(this.pos, MAGE_RANGE) {
                                enemy.try_write().unwrap().active = true;
                            }
                        }
                    }
                    2 => {
                        // spell time
                        if board[player.pos].is_none() {
                            this.windup = 3;
                            this.variant = Variant::Mage(Spell::Circle(player.pos));
                        }
                    }
                    3 => {
                        // do nothing
                    }
                    _ => unreachable!("Bit and seems to be broken"),
                }
            }
            Variant::BasicBoss(direction) => {
                if this.windup > 0 {
                    if this.windup == 1 {
                        // charge time
                        let mut pos = this.pos;
                        loop {
                            pos += direction;
                            if player.pos == pos {
                                let _ = player
                                    .attacked(20, Variant::BasicBoss(Direction::Up).kill_name());
                                break;
                            }
                            if let Some(piece) = &board[pos] {
                                if piece.enemy_collision() {
                                    break;
                                }
                            }
                            let mut other = None;
                            for enemy in board.enemies.iter() {
                                if Arc::ptr_eq(enemy, &arc) {
                                    continue;
                                }
                                if enemy.try_read().unwrap().pos == pos {
                                    other = Some(enemy.clone());
                                    break;
                                }
                            }
                            if let Some(other) = other {
                                albatrice::debug!(if Arc::ptr_eq(&other, &arc) {
                                    unreachable!("basic boss charged itself")
                                });
                                other.try_write().unwrap().attacked();
                                other.try_write().unwrap().attacked();
                                break;
                            }
                        }
                        this.pos = pos - direction;
                        this.windup = 0;
                    } else {
                        this.windup -= 1;
                    }
                } else if this.is_near(player.pos, 2) {
                    // smack 'em
                    let _ = player.attacked(
                        (crate::random() & 0b11) as usize + 3,
                        Variant::BasicBoss(Direction::Up).kill_name(),
                    );
                } else if this.pos.x == player.pos.x {
                    // charge up a vertical charge
                    if this.pos.y > player.pos.y {
                        this.variant = Variant::BasicBoss(Direction::Up)
                    } else {
                        this.variant = Variant::BasicBoss(Direction::Down)
                    }
                    this.windup = 2;
                } else if this.pos.y == player.pos.y {
                    // charge up a horizontal charge
                    if this.pos.x > player.pos.x {
                        this.variant = Variant::BasicBoss(Direction::Left)
                    } else {
                        this.variant = Variant::BasicBoss(Direction::Right)
                    }
                    this.windup = 2;
                }
            }
            Variant::MageBoss => {}
        }
    }
    pub fn is_near(&self, pos: Vector, range: usize) -> bool {
        self.pos.x.abs_diff(pos.x) < range && self.pos.y.abs_diff(pos.y) < range
    }
    pub fn promote(&mut self) -> Result<(), ()> {
        match self.variant {
            Variant::Basic => self.variant = Variant::BasicBoss(Direction::Up),
            Variant::BasicBoss(_) => return Err(()),
            Variant::Mage(_) => self.variant = Variant::MageBoss,
            Variant::MageBoss => return Err(()),
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Variant {
    Basic,
    BasicBoss(Direction),
    Mage(Spell),
    MageBoss,
}
impl Variant {
    fn detect(self, enemy: &RwLockWriteGuard<Enemy>, board: &Board) -> bool {
        match self {
            Variant::Basic => match board.backtraces[board.x * enemy.pos.y + enemy.pos.x].cost {
                Some(cost) => cost < (crate::random() & 0b0000_0111) as usize,
                None => false,
            },
            Variant::Mage(_) => match board.backtraces[board.x * enemy.pos.y + enemy.pos.x].cost {
                Some(cost) => cost < (((crate::random() & 0b0000_0111) + 1) << 2) as usize,
                None => false,
            },
            Variant::BasicBoss(_) => board.backtraces[board.x * enemy.pos.y + enemy.pos.x]
                .cost
                .is_some(),
            Variant::MageBoss => board.backtraces[board.x * enemy.pos.y + enemy.pos.x]
                .cost
                .is_some(),
        }
    }
    fn windup_color(self) -> Color {
        // red is physical
        // purple is magic
        match self {
            Variant::Basic => Color::Red,
            Variant::BasicBoss(_) => Color::Red,
            Variant::Mage(_) => Color::Purple,
            Variant::MageBoss => Color::Purple,
        }
    }
    fn max_health(self) -> usize {
        match self {
            Variant::Basic => 3,
            Variant::BasicBoss(_) => 10,
            Variant::Mage(_) => 5,
            Variant::MageBoss => 10,
        }
    }
    fn parry_stun(self) -> usize {
        match self {
            Variant::Basic => 3,
            Variant::BasicBoss(_) => 1,
            Variant::Mage(_) => 0,
            Variant::MageBoss => 0,
        }
    }
    fn dash_stun(self) -> usize {
        match self {
            Variant::Basic => 1,
            Variant::BasicBoss(_) => 0,
            Variant::Mage(_) => 2,
            Variant::MageBoss => 0,
        }
    }
    // returns kill reward in energy, then health
    // per energy
    pub const fn kill_value(self) -> (usize, usize) {
        match self {
            Variant::Basic => (1, 5),
            Variant::BasicBoss(_) => (10, 10),
            Variant::Mage(_) => (5, 5),
            Variant::MageBoss => (20, 5),
        }
    }
    pub fn kill_name(self) -> &'static str {
        match self {
            Variant::Basic => "Repurposed Automata",
            Variant::BasicBoss(_) => "Specialized Automata",
            Variant::Mage(_) => "Mage Construct",
            Variant::MageBoss => "",
        }
    }
    pub fn is_boss(self) -> bool {
        match self {
            Variant::Basic => false,
            Variant::BasicBoss(_) => true,
            Variant::Mage(_) => false,
            Variant::MageBoss => true,
        }
    }
    // used to get which type should be promoted into the boss
    pub fn get_tier(self) -> Result<usize, ()> {
        match self {
            Variant::Basic => Ok(1),
            Variant::BasicBoss(_) => Err(()),
            Variant::Mage(_) => Ok(2),
            Variant::MageBoss => Err(()),
        }
    }
}
impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variant::Basic => write!(f, "basic"),
            Variant::Mage(_) => write!(f, "mage"),
            Variant::BasicBoss(_) => write!(f, "basic_boss"),
            Variant::MageBoss => write!(f, "mage_boss"),
        }
    }
}
impl std::str::FromStr for Variant {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Variant::Basic),
            "mage" => Ok(Variant::Mage(Spell::Teleport)),
            "basic_boss" => Ok(Variant::BasicBoss(Direction::Up)),
            "mage_boss" => Ok(Variant::MageBoss),
            _ => Err("invalid variant".to_string()),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Spell {
    Circle(Vector),
    Teleport,
}
