use crate::{Board, Direction, Player, Style, Vector, board::Special, pieces::spell, style::Color};
use std::sync::{Arc, RwLock, RwLockWriteGuard, Weak};
const MAGE_RANGE: usize = 30;
const TELE_THRESH: usize = 5;
const MAGE_BOSS_PROMOTE_RANGE: usize = 10;
const MAGE_BOSS_SWAP_RANGE: usize = 20;
const MAGE_BOSS_SWAP_THRESH: usize = 10;
#[derive(Clone, Debug)]
pub struct Enemy {
    pub health: usize,
    pub variant: Variant,
    stun: usize,
    windup: usize,
    pub pos: Vector,
    pub active: bool,
    pub reachable: bool,
    pub attacking: bool,
    pub dead: bool,
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
            dead: false,
        }
    }
    pub fn render(&self) -> (char, Option<crate::Style>) {
        (
            match self.variant {
                Variant::Basic | Variant::BasicBoss(_) => '1',
                Variant::Mage(_) | Variant::MageBoss(_) => '2',
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
                if self.is_stunned() {
                    out.background_blue();
                } else if self.is_windup() {
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
    pub fn attacked(&mut self, damage: usize) -> bool {
        if damage > self.health {
            self.dead = true
        } else {
            self.health -= damage;
            if self.health == 0 {
                self.dead = true
            }
        }
        self.active = true;
        self.dead
    }
    // returns whether or not it needs to re-render the board after this
    pub fn think(arc: Arc<RwLock<Self>>, board: &mut Board, player: &mut Player) -> bool {
        let mut this = arc.try_write().unwrap();
        let addr = Arc::as_ptr(&arc).addr();
        if !this.active {
            if this.variant.detect(&this, board) {
                this.active = true;
            } else {
                return false;
            }
        }
        if this.stun != 0 {
            this.stun -= 1;
            return false;
        }
        match this.variant.clone() {
            Variant::Basic => {
                if player.pos.x.abs_diff(this.pos.x) < 2 && player.pos.y.abs_diff(this.pos.y) < 2 {
                    this.attacking = true;
                    if this.windup == 0 {
                        this.windup = 3;
                        return true;
                    }
                    this.windup -= 1;
                    if this.windup == 0 {
                        if let Err(_) = player.attacked(
                            (crate::random() & 0b0000_0011) as usize + 3,
                            Variant::Basic.kill_name(),
                        ) {
                            this.stun = this.variant.parry_stun();
                        }
                        return true;
                    }
                    false
                } else {
                    this.attacking = false;
                    this.windup = 0;
                    false
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
                    return false;
                }
                if this.windup == 1 {
                    // cast time BAYBEEE
                    match spell {
                        MageSpell::Circle(cast_pos) => {
                            if board[cast_pos].is_none() {
                                board[cast_pos] = Some(crate::board::Piece::Spell(
                                    spell::Spell::new(Arc::downgrade(&arc)),
                                ));
                            }
                            this.windup = 0;
                        }
                        MageSpell::Teleport => {
                            if let Some(target) = board
                                .pick_near(Some(addr), this.pos, MAGE_RANGE)
                                .map(|weak| weak.upgrade().unwrap())
                            {
                                std::mem::swap(&mut target.try_write().unwrap().pos, &mut this.pos);
                                crate::RE_FLOOD.store(true, std::sync::atomic::Ordering::Relaxed);
                            }
                            this.windup = 0;
                        }
                    }
                }
                match crate::random() & 0b0000_0011 {
                    0 => {
                        // teleport
                        if this.is_near(player.pos, TELE_THRESH) {
                            this.windup = 3;
                            this.variant = Variant::Mage(MageSpell::Teleport);
                            return true;
                        }
                        false
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
                        false
                    }
                    2 => {
                        // spell time
                        if board[player.pos].is_none() {
                            this.windup = 3;
                            this.variant = Variant::Mage(MageSpell::Circle(player.pos));
                        }
                        true
                    }
                    3 => {
                        // do nothing
                        false
                    }
                    _ => unreachable!("Bit and seems to be broken"),
                }
            }
            Variant::BasicBoss(direction) => {
                if this.windup > 0 {
                    if this.windup == 1 {
                        // charge time
                        let mut pos = this.pos;
                        let mut this = Some(this);
                        // Explained lower
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
                                other.try_write().unwrap().attacked(4);
                                break;
                            }
                            this.as_mut().unwrap().pos = pos;
                            // It is VERY important that we release the lock on the enemy here.
                            // Rendering REQUIRES that it can get a read of EVERY enemy (that
                            // includes this one) which means that even though I have to do bad
                            // shit, it is very important that the value inside this is dropped.
                            this.take();
                            board.smart_render(player);
                            this = Some(arc.try_write().unwrap());
                            std::thread::sleep(crate::PROJ_DELAY);
                        }
                        let mut this = this.unwrap();
                        this.pos = pos - direction;
                        this.windup = 0;
                        this.attacking = false;
                        true
                    } else {
                        this.windup -= 1;
                        false
                    }
                } else if this.is_near(player.pos, 2) {
                    // smack 'em
                    let _ = player.attacked(
                        (crate::random() & 0b11) as usize + 3,
                        Variant::BasicBoss(Direction::Up).kill_name(),
                    );
                    true
                } else if this.pos.x == player.pos.x {
                    // charge up a vertical charge
                    if this.pos.y > player.pos.y {
                        this.variant = Variant::BasicBoss(Direction::Up)
                    } else {
                        this.variant = Variant::BasicBoss(Direction::Down)
                    }
                    this.windup = 2;
                    this.attacking = true;
                    true
                } else if this.pos.y == player.pos.y {
                    // charge up a horizontal charge
                    if this.pos.x > player.pos.x {
                        this.variant = Variant::BasicBoss(Direction::Left)
                    } else {
                        this.variant = Variant::BasicBoss(Direction::Right)
                    }
                    this.windup = 2;
                    this.attacking = true;
                    true
                } else {
                    false
                }
            }
            Variant::MageBoss(spell) => {
                if this.windup > 0 {
                    let mut this = Some(this);
                    if this.as_ref().unwrap().windup == 1 {
                        // casting time
                        this.as_mut().unwrap().attacking = false;
                        match spell {
                            MageBossSpell::Obamehameha(direction) => {
                                let mut pos = this.as_ref().unwrap().pos;
                                crate::log!("firing obamehameha from {pos} to {direction}");
                                this.take();
                                'outer: loop {
                                    pos += direction;
                                    crate::log!("\tchecking {pos}");
                                    if let Some(piece) = &board[pos] {
                                        crate::log!("\t\thit piece: {piece}");
                                        if piece.has_collision() {
                                            break;
                                        }
                                    }
                                    if player.pos == pos {
                                        crate::log!("\t\thit player");
                                        break;
                                    }
                                    for enemy in board.enemies.iter() {
                                        if Arc::as_ptr(enemy).addr() == addr {
                                            continue;
                                        }
                                        if enemy.try_read().unwrap().pos == pos {
                                            crate::log!("\t\thit enemy");
                                            break 'outer;
                                        }
                                    }
                                    board.specials.push(Special::new(
                                        pos,
                                        '●',
                                        Some(*Style::new().red().intense(true)),
                                    ));
                                    board.smart_render(player);
                                    board.specials.pop();
                                    std::thread::sleep(crate::PROJ_DELAY);
                                }
                                pos = pos - direction;

                                let restore_to = board.specials.len();
                                board.specials.push(Special::new(
                                    pos,
                                    ' ',
                                    Some(*Style::new().background_red()),
                                ));
                                board.smart_render(player);
                                board.specials.pop();
                                std::thread::sleep(crate::DELAY * 2);
                                for x in -1..=1 {
                                    for y in -1..=1 {
                                        board.specials.push(Special::new(
                                            Vector::new(
                                                (pos.x as isize + x) as usize,
                                                (pos.y as isize + y) as usize,
                                            ),
                                            ' ',
                                            Some(*Style::new().background_red()),
                                        ));
                                    }
                                }
                                board.smart_render(player);
                                board.specials.truncate(restore_to);

                                std::thread::sleep(crate::DELAY * 2);

                                for enemy in board
                                    .get_near(None, pos, 3)
                                    .iter()
                                    .map(|enemy| enemy.upgrade().unwrap())
                                {
                                    enemy
                                        .try_write()
                                        .unwrap()
                                        .attacked((crate::random() as usize & 3) + 1);
                                }
                                if player.pos.is_near(pos, 3) {
                                    let damage = (((crate::random() & 3) + 1) * 10) as usize;
                                    crate::log!("\tDamaging player for {damage}");
                                    let _ =
                                        player.attacked(damage, Variant::mage_boss().kill_name());
                                }
                                this = Some(arc.try_write().unwrap());
                            }
                            MageBossSpell::Promote(enemy) => {
                                if let Some(enemy) = enemy.upgrade() {
                                    enemy.try_write().unwrap().variant = Variant::mage();
                                }
                            }
                            MageBossSpell::Create => {
                                // If the player is near then get_adjacent can crash or fail
                                if this.as_ref().unwrap().is_near(player.pos, 5) {
                                    this.unwrap().windup = 0;
                                    return false;
                                }
                                'outer: for pos in board
                                    .get_adjacent(this.as_ref().unwrap().pos, None, true)
                                    .to_vec(this.as_ref().unwrap().pos)
                                    .iter()
                                {
                                    for enemy in board.enemies.iter() {
                                        if Arc::as_ptr(enemy).addr() == addr {
                                            continue;
                                        }
                                        if enemy.try_read().unwrap().pos == *pos {
                                            continue 'outer;
                                        }
                                    }
                                    board.enemies.push(Arc::new(RwLock::new(Enemy::new(
                                        *pos,
                                        Variant::Basic,
                                    ))));
                                    crate::RE_FLOOD
                                        .store(true, std::sync::atomic::Ordering::Relaxed);
                                    break;
                                }
                            }
                            MageBossSpell::Swap(enemy) => {
                                if let Some(enemy) = enemy.upgrade() {
                                    std::mem::swap(
                                        &mut this.as_mut().unwrap().pos,
                                        &mut enemy.try_write().unwrap().pos,
                                    );
                                }
                                crate::RE_FLOOD.store(true, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                    }
                    this.as_mut().unwrap().windup -= 1;
                    // redraw if it actually cast something
                    this.unwrap().windup == 0
                }
                // Deciding what to do
                else {
                    if (this.pos.x == player.pos.x || this.pos.y == player.pos.y)
                        && this.is_near(player.pos, 15)
                    {
                        // Obamehameha
                        if crate::random() & 3 == 0 {
                            let dir;
                            if this.pos.x == player.pos.x {
                                if this.pos.y > player.pos.y {
                                    dir = Direction::Up;
                                } else {
                                    dir = Direction::Down
                                }
                            } else {
                                if this.pos.x > player.pos.x {
                                    dir = Direction::Left
                                } else {
                                    dir = Direction::Right
                                }
                            }
                            this.variant = Variant::MageBoss(MageBossSpell::Obamehameha(dir));
                            this.windup = 4;
                            this.attacking = true;
                            return true;
                        }
                        false
                    } else {
                        match crate::random() & 3 {
                            0 => {
                                // Promote
                                let mut candidates = Vec::new();
                                for enemy in board.enemies.iter() {
                                    if Arc::as_ptr(enemy).addr() == addr {
                                        continue;
                                    }
                                    if let Variant::Basic = enemy.try_read().unwrap().variant {
                                        if enemy
                                            .try_read()
                                            .unwrap()
                                            .is_near(this.pos, MAGE_BOSS_PROMOTE_RANGE)
                                        {
                                            candidates.push(Arc::downgrade(enemy))
                                        }
                                    }
                                }
                                if let Some(chosen) = crate::random::random_index(candidates.len())
                                    .map(|index| candidates.swap_remove(index))
                                {
                                    this.variant =
                                        Variant::MageBoss(MageBossSpell::Promote(chosen));
                                    this.windup = 5;
                                    this.attacking = true;
                                    return true;
                                }
                                false
                            }
                            1 => {
                                // Create
                                this.variant = Variant::MageBoss(MageBossSpell::Create);
                                this.windup = 5;
                                this.attacking = true;
                                true
                            }
                            2 => {
                                // Swap
                                if !this.is_near(player.pos, MAGE_BOSS_SWAP_THRESH) {
                                    return false;
                                }
                                if let Some(chosen) =
                                    board.pick_near(Some(addr), this.pos, MAGE_BOSS_SWAP_RANGE)
                                {
                                    this.variant = Variant::MageBoss(MageBossSpell::Swap(chosen));
                                    this.windup = 5;
                                    this.attacking = true;
                                    return true;
                                }
                                false
                            }
                            3 => false,
                            _ => unreachable!("Shit -> Fan"),
                        }
                    }
                }
            }
        }
    }
    pub fn is_near(&self, pos: Vector, range: usize) -> bool {
        self.pos.x.abs_diff(pos.x) < range && self.pos.y.abs_diff(pos.y) < range
    }
    pub fn promote(&mut self) -> Result<(), ()> {
        match self.variant {
            Variant::Basic => self.variant = Variant::basic_boss(),
            Variant::BasicBoss(_) => return Err(()),
            Variant::Mage(_) => self.variant = Variant::mage_boss(),
            Variant::MageBoss(_) => return Err(()),
        }
        Ok(())
    }
    pub fn alert_nearby(&self, addr: usize, board: &Board, range: usize) {
        for enemy in board.enemies.iter() {
            if Arc::as_ptr(enemy).addr() == addr {
                continue;
            }
            let mut enemy = enemy.try_write().unwrap();
            if enemy.is_near(self.pos, range) {
                enemy.active = true;
            }
        }
    }
}
#[derive(Clone, Debug)]
pub enum Variant {
    Basic,
    BasicBoss(Direction),
    Mage(MageSpell),
    MageBoss(MageBossSpell),
}
impl Variant {
    fn detect(&self, enemy: &RwLockWriteGuard<Enemy>, board: &Board) -> bool {
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
            Variant::MageBoss(_) => board.backtraces[board.x * enemy.pos.y + enemy.pos.x]
                .cost
                .is_some(),
        }
    }
    fn windup_color(&self) -> Color {
        // red is physical
        // purple is magic
        match self {
            Variant::Basic => Color::Red,
            Variant::BasicBoss(_) => Color::Red,
            Variant::Mage(_) => Color::Purple,
            Variant::MageBoss(_) => Color::Purple,
        }
    }
    fn max_health(&self) -> usize {
        match self {
            Variant::Basic => 3,
            Variant::BasicBoss(_) => 10,
            Variant::Mage(_) => 5,
            Variant::MageBoss(_) => 10,
        }
    }
    fn parry_stun(&self) -> usize {
        match self {
            Variant::Basic => 3,
            Variant::BasicBoss(_) => 1,
            Variant::Mage(_) => 0,
            Variant::MageBoss(_) => 0,
        }
    }
    fn dash_stun(&self) -> usize {
        match self {
            Variant::Basic => 1,
            Variant::BasicBoss(_) => 0,
            Variant::Mage(_) => 2,
            Variant::MageBoss(_) => 0,
        }
    }
    // returns kill reward in energy, then health
    // per energy
    pub const fn kill_value(&self) -> (usize, usize) {
        match self {
            Variant::Basic => (1, 5),
            Variant::BasicBoss(_) => (10, 10),
            Variant::Mage(_) => (5, 5),
            Variant::MageBoss(_) => (20, 5),
        }
    }
    pub fn kill_name(&self) -> &'static str {
        match self {
            Variant::Basic => "Repurposed Automata",
            Variant::BasicBoss(_) => "Specialized Automata",
            Variant::Mage(_) => "Mage Construct",
            Variant::MageBoss(_) => "Lazy Mage",
        }
    }
    pub fn is_boss(&self) -> bool {
        match self {
            Variant::Basic => false,
            Variant::BasicBoss(_) => true,
            Variant::Mage(_) => false,
            Variant::MageBoss(_) => true,
        }
    }
    // used to get which type should be promoted into the boss
    pub fn get_tier(&self) -> Result<usize, ()> {
        match self {
            Variant::Basic => Ok(1),
            Variant::BasicBoss(_) => Err(()),
            Variant::Mage(_) => Ok(2),
            Variant::MageBoss(_) => Err(()),
        }
    }
    pub const fn basic() -> Variant {
        Variant::Basic
    }
    pub const fn basic_boss() -> Variant {
        Variant::BasicBoss(Direction::Up)
    }
    pub const fn mage() -> Variant {
        Variant::Mage(MageSpell::Teleport)
    }
    pub const fn mage_boss() -> Variant {
        Variant::MageBoss(MageBossSpell::Create)
    }
}
impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variant::Basic => write!(f, "basic"),
            Variant::Mage(_) => write!(f, "mage"),
            Variant::BasicBoss(_) => write!(f, "basic_boss"),
            Variant::MageBoss(_) => write!(f, "mage_boss"),
        }
    }
}
impl std::str::FromStr for Variant {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Variant::Basic),
            "mage" => Ok(Variant::mage()),
            "basic_boss" => Ok(Variant::basic_boss()),
            "mage_boss" => Ok(Variant::mage_boss()),
            _ => Err("invalid variant".to_string()),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum MageSpell {
    Circle(Vector),
    Teleport,
}
#[derive(Clone, Debug)]
pub enum MageBossSpell {
    Obamehameha(Direction),
    // promote basic to mage (5 turns)
    Promote(Weak<RwLock<Enemy>>),
    // create new basic (10 turns)
    Create,
    // swap places with another enemy (5 turns)
    Swap(Weak<RwLock<Enemy>>),
}
