use crate::{
    Board, Direction, Player, Vector, advantage_pass,
    random::*,
    spell::{ContactSpell, NormalSpell, Spell, SpellCircle},
    style::Color,
};
use std::io::Write;
use std::sync::{Arc, RwLock, RwLockWriteGuard, Weak};
const MAGE_RANGE: usize = 30;
const TELE_THRESH: usize = 5;
const MAGE_BOSS_PROMOTE_RANGE: usize = 10;
const MAGE_BOSS_SWAP_THRESH: usize = 10;
#[derive(Debug)]
pub struct Enemy {
    pub health: usize,
    pub variant: Variant,
    stun: usize,
    windup: usize,
    pub pos: Vector,
    pub active: bool,
    pub attacking: bool,
    pub dead: bool,
    // If it should give rewards when killed
    pub reward: bool,
    pub log: Option<std::fs::File>,
    pub show_line_of_sight: bool,
    // If it took damage last turn
    pub took_damage: bool,
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
            attacking: false,
            dead: false,
            reward: true,
            log: None,
            show_line_of_sight: false,
            took_damage: false,
        }
    }
    pub fn render(&self) -> (char, Option<crate::Style>) {
        (
            match self.variant {
                Variant::Basic | Variant::BasicBoss(_) => '1',
                Variant::Mage(_) | Variant::MageBoss(_) => '2',
                Variant::Fighter(_) | Variant::FighterBoss { .. } => '4',
                Variant::Archer(_) | Variant::ArcherBoss(_) => '3',
            },
            Some({
                let mut out = crate::Style::new();
                if self.variant.is_boss() {
                    out.blue().intense(true);
                } else if self.active {
                    out.yellow();
                }
                if self.is_stunned() {
                    out.background_blue();
                } else if self.is_windup() {
                    out.set_background(self.variant.windup_color())
                        .intense_background(true);
                }
                if self.took_damage {
                    out.italic(true);
                    /*if out.has_background() {
                        out.swap_grounds();
                    } else {
                        out.dim(true);
                    }*/
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
    pub fn attacked(&mut self, mut damage: usize) -> bool {
        self.log(format!("Attacked for {damage} damage"));
        if self.is_stunned() {
            damage += 1;
            self.log(format!("  Is stunned, increasing damage to {damage}"));
        }
        if damage >= self.health {
            self.log("  And died".to_string());
            self.dead = true
        } else {
            self.health -= damage;
        }
        if !self.active {
            self.log("Waking up due to attack".to_string());
        }
        self.active = true;
        self.took_damage = true;
        self.dead
    }
    // returns whether or not it needs to re-render the board after this
    pub fn think(
        arc: Arc<RwLock<Self>>,
        board: &mut Board,
        player: &mut Player,
        time: &mut std::time::Duration,
    ) -> bool {
        let mut start = std::time::Instant::now();
        let mut this = Some(arc.try_write().unwrap());
        if !board.is_reachable(this.as_ref().unwrap().pos) {
            *time += start.elapsed();
            return false;
        }
        let addr = Arc::as_ptr(&arc).addr();
        if !this.as_ref().unwrap().active {
            log(&mut this, "Attempting wake up".to_string());
            if this.as_ref().unwrap().variant.mage_aggro() && player.limbs.count_mage_eyes() > 0 {
                this.as_mut()
                    .unwrap()
                    .log("Woke up due to mage sight".to_string());
                this.as_mut().unwrap().active = true;
            } else if this
                .as_ref()
                .unwrap()
                .variant
                .detect(this.as_ref().unwrap(), board, player)
            {
                this.as_mut()
                    .unwrap()
                    .log("Woke up due to detection".to_string());
                this.as_mut().unwrap().active = true;
            } else {
                *time += start.elapsed();
                return false;
            }
        }
        if this.as_ref().unwrap().stun != 0 {
            this.as_mut().unwrap().stun -= 1;
            *time += start.elapsed();
            return false;
        }
        let pos = this.as_ref().unwrap().pos;
        let player_pos = player.pos;
        let show_line_of_sight = this.as_ref().unwrap().show_line_of_sight;
        // put in a lazycell because that way the expensive ray cast is only done when needed
        let line_of_sight = std::cell::LazyCell::new(|| {
            let (ray, collision) = crate::ray_cast(
                pos,
                player_pos,
                board,
                Some(Arc::as_ptr(&arc).addr()),
                false,
                player_pos,
            );
            if let Some(crate::Collision::Player) = collision {
                if show_line_of_sight {
                    for pos in ray.into_iter() {
                        board.add_one_turn_special(crate::board::Special::new(
                            pos,
                            ' ',
                            Some(*crate::Style::new().background_green()),
                        ));
                    }
                }
                true
            } else {
                if show_line_of_sight {
                    for pos in ray.into_iter() {
                        board.add_one_turn_special(crate::board::Special::new(
                            pos,
                            ' ',
                            Some(*crate::Style::new().background_red()),
                        ));
                    }
                }
                false
            }
        });
        match this.as_ref().unwrap().variant.clone() {
            Variant::Basic => {
                if player.pos.x.abs_diff(this.as_ref().unwrap().pos.x) < 2
                    && player.pos.y.abs_diff(this.as_ref().unwrap().pos.y) < 2
                {
                    this.as_mut().unwrap().attacking = true;
                    if this.as_ref().unwrap().windup == 0 {
                        this.as_mut().unwrap().windup = 2;
                        *time += start.elapsed();
                        return true;
                    }
                    this.as_mut().unwrap().windup -= 1;
                    if this.as_ref().unwrap().windup == 0 {
                        if player
                            .attacked(
                                luck_roll8(player) as usize + 3,
                                Variant::Basic.kill_name(),
                                Some(Variant::basic().to_key()),
                            )
                            .is_err()
                        {
                            this.as_mut().unwrap().stun =
                                this.as_ref().unwrap().variant.parry_stun();
                        }
                        *time += start.elapsed();
                        return true;
                    }
                    *time += start.elapsed();
                    false
                } else {
                    this.as_mut().unwrap().attacking = false;
                    this.as_mut().unwrap().windup = 0;
                    *time += start.elapsed();
                    false
                }
            }
            Variant::Mage(spell) => {
                if this.as_ref().unwrap().is_near(player.pos, MAGE_RANGE) {
                    this.as_mut().unwrap().attacking = true;
                } else {
                    this.as_mut().unwrap().attacking = false;
                    this.as_mut().unwrap().windup = 0;
                }
                if this.as_ref().unwrap().windup > 1 {
                    this.as_mut().unwrap().windup -= 1;
                    *time += start.elapsed();
                    return false;
                }
                if this.as_ref().unwrap().windup == 1 {
                    // cast time BAYBEEE
                    match spell {
                        MageSpell::Circle(cast_pos) => {
                            crate::log!("  Casting circle");
                            if !board.contains_literally_anything(cast_pos, Some(addr)) {
                                board.spells.push(SpellCircle {
                                    spell: Spell::Contact(ContactSpell::DrainHealth),
                                    pos: cast_pos,
                                    caster: Some(arc.clone()),
                                    aim: None,
                                });
                            }
                            this.as_mut().unwrap().windup = 0;
                            *time += start.elapsed();
                        }
                        MageSpell::Teleport => {
                            crate::log!("  Casting swap");
                            assert!(arc.try_write().is_err(), "RUH ROH RAGGY");
                            this.take();
                            assert!(arc.try_write().is_ok(), "AR NAWR");
                            *time += start.elapsed();
                            NormalSpell::Swap.cast(
                                Some(arc.clone()),
                                player,
                                board,
                                None,
                                None,
                                Some(time),
                            );
                            start = std::time::Instant::now();
                            this = Some(arc.try_write().unwrap());
                            this.as_mut().unwrap().windup = 0;
                        }
                    }
                }
                let mut this = this.unwrap();
                match crate::random() & 0b0000_0011 {
                    0 => {
                        // teleport
                        if this.is_near(player.pos, TELE_THRESH) {
                            this.windup = NormalSpell::Swap.cast_time();
                            this.variant = Variant::Mage(MageSpell::Teleport);
                            *time += start.elapsed();
                            return true;
                        }
                        *time += start.elapsed();
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
                        *time += start.elapsed();
                        false
                    }
                    2 => {
                        // spell time
                        if board[player.pos].is_none() {
                            this.windup = ContactSpell::DrainHealth.cast_time();
                            this.variant = Variant::Mage(MageSpell::Circle(player.pos));
                        }
                        *time += start.elapsed();
                        true
                    }
                    3 => {
                        // do nothing
                        *time += start.elapsed();
                        false
                    }
                    _ => unreachable!("Bit and seems to be broken"),
                }
            }
            Variant::BasicBoss(direction) => {
                if this.as_ref().unwrap().windup > 0 {
                    if this.as_ref().unwrap().windup == 1 {
                        this.as_mut().unwrap().log(format!("Charging {direction}"));
                        let start_pos = this.as_ref().unwrap().pos;
                        // charge time
                        this.take();
                        *time += start.elapsed();
                        NormalSpell::Charge.cast(
                            Some(arc.clone()),
                            player,
                            board,
                            None,
                            Some(start_pos + direction),
                            Some(time),
                        );
                        this = Some(arc.try_write().unwrap());
                        this.as_mut().unwrap().windup = 0;
                        true
                    } else {
                        this.as_mut().unwrap().windup -= 1;
                        let new_windup = this.as_ref().unwrap().windup;
                        this.as_mut()
                            .unwrap()
                            .log(format!("Decrimenting windup, now at {new_windup}"));
                        *time += start.elapsed();
                        false
                    }
                } else if this.as_ref().unwrap().is_near(player.pos, 2) {
                    // smack 'em
                    let damage = luck_roll8(player) as usize / 2 + 3;
                    this.as_mut()
                        .unwrap()
                        .log(format!("Attacking player for {damage}"));
                    let _ = player.attacked(
                        damage,
                        Variant::BasicBoss(Direction::Up).kill_name(),
                        Some(Variant::basic_boss().to_key()),
                    );
                    *time += start.elapsed();
                    true
                } else if this.as_ref().unwrap().pos.x == player.pos.x && *line_of_sight {
                    // charge up a vertical charge
                    if this.as_ref().unwrap().pos.y > player.pos.y {
                        this.as_mut().unwrap().log("Starting charge up".to_string());
                        this.as_mut().unwrap().variant = Variant::BasicBoss(Direction::Up)
                    } else {
                        this.as_mut()
                            .unwrap()
                            .log("Starting charge down".to_string());
                        this.as_mut().unwrap().variant = Variant::BasicBoss(Direction::Down)
                    }
                    this.as_mut().unwrap().windup = 2;
                    this.as_mut().unwrap().attacking = true;
                    *time += start.elapsed();
                    true
                } else if this.as_ref().unwrap().pos.y == player.pos.y && *line_of_sight {
                    // charge up a horizontal charge
                    if this.as_ref().unwrap().pos.x > player.pos.x {
                        this.as_mut()
                            .unwrap()
                            .log("Starting charge left".to_string());
                        this.as_mut().unwrap().variant = Variant::BasicBoss(Direction::Left)
                    } else {
                        this.as_mut()
                            .unwrap()
                            .log("Starting charge right".to_string());
                        this.as_mut().unwrap().variant = Variant::BasicBoss(Direction::Right)
                    }
                    this.as_mut().unwrap().windup = 2;
                    this.as_mut().unwrap().attacking = true;
                    *time += start.elapsed();
                    true
                } else {
                    this.as_mut().unwrap().log("Doing nothing".to_string());
                    this.as_mut().unwrap().attacking = false;
                    *time += start.elapsed();
                    false
                }
            }
            Variant::MageBoss(spell) => {
                if this.as_ref().unwrap().windup > 0 {
                    if this.as_ref().unwrap().windup == 1 {
                        // casting time
                        this.as_mut().unwrap().attacking = false;
                        match spell {
                            MageBossSpell::Obamehameha(direction) => {
                                let aim = this.as_ref().unwrap().pos + direction;
                                this.take();
                                *time += start.elapsed();
                                NormalSpell::BidenBlast.cast(
                                    Some(arc.clone()),
                                    player,
                                    board,
                                    None,
                                    Some(aim),
                                    Some(time),
                                );
                                start = std::time::Instant::now();
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
                                    *time += start.elapsed();
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
                                    board.enemies.push(Arc::new(RwLock::new(
                                        Enemy::new(*pos, Variant::Basic).set_reward(false),
                                    )));
                                    crate::RE_FLOOD
                                        .store(true, std::sync::atomic::Ordering::Relaxed);
                                    break;
                                }
                            }
                            MageBossSpell::Swap => {
                                this.take();
                                NormalSpell::Swap.cast(
                                    Some(arc.clone()),
                                    player,
                                    board,
                                    None,
                                    None,
                                    Some(time),
                                );
                                this = Some(arc.try_write().unwrap());
                            }
                        }
                    }
                    this.as_mut().unwrap().windup -= 1;
                    // redraw if it actually cast something
                    *time += start.elapsed();
                    this.unwrap().windup == 0
                }
                // Deciding what to do
                else if (this.as_ref().unwrap().pos.x == player.pos.x
                    || this.as_ref().unwrap().pos.y == player.pos.y)
                    && this.as_ref().unwrap().is_near(player.pos, 15)
                {
                    // Obamehameha
                    if crate::random() & 3 == 0 && *line_of_sight {
                        let dir;
                        if this.as_ref().unwrap().pos.x == player.pos.x {
                            if this.as_ref().unwrap().pos.y > player.pos.y {
                                dir = Direction::Up;
                            } else {
                                dir = Direction::Down
                            }
                        } else if this.as_ref().unwrap().pos.x > player.pos.x {
                            dir = Direction::Left
                        } else {
                            dir = Direction::Right
                        }
                        this.as_mut().unwrap().variant =
                            Variant::MageBoss(MageBossSpell::Obamehameha(dir));
                        this.as_mut().unwrap().windup = 4;
                        this.as_mut().unwrap().attacking = true;
                        *time += start.elapsed();
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
                                    if enemy.try_read().unwrap().is_near(
                                        this.as_ref().unwrap().pos,
                                        MAGE_BOSS_PROMOTE_RANGE,
                                    ) {
                                        candidates.push(Arc::downgrade(enemy))
                                    }
                                }
                            }
                            if let Some(chosen) = crate::random::random_index(candidates.len())
                                .map(|index| candidates.swap_remove(index))
                            {
                                this.as_mut().unwrap().variant =
                                    Variant::MageBoss(MageBossSpell::Promote(chosen));
                                this.as_mut().unwrap().windup = 5;
                                this.as_mut().unwrap().attacking = true;
                                *time += start.elapsed();
                                return true;
                            }
                            *time += start.elapsed();
                            false
                        }
                        1 => {
                            // Create
                            this.as_mut().unwrap().variant =
                                Variant::MageBoss(MageBossSpell::Create);
                            this.as_mut().unwrap().windup = 5;
                            this.as_mut().unwrap().attacking = true;
                            *time += start.elapsed();
                            true
                        }
                        2 => {
                            // Swap
                            if !this
                                .as_ref()
                                .unwrap()
                                .is_near(player.pos, MAGE_BOSS_SWAP_THRESH)
                            {
                                return false;
                            }
                            this.as_mut().unwrap().variant = Variant::MageBoss(MageBossSpell::Swap);
                            this.as_mut().unwrap().windup = 5;
                            this.as_mut().unwrap().attacking = true;
                            *time += start.elapsed();
                            true
                        }
                        3 => false,
                        _ => unreachable!("Shit -> Fan"),
                    }
                }
            }
            Variant::Fighter(action) => {
                if this.as_ref().unwrap().windup > 0 {
                    if this.as_ref().unwrap().windup == 1 {
                        // doin time
                        match action {
                            FighterAction::Smack => {
                                if this.as_ref().unwrap().pos.is_near(player.pos, 2) {
                                    let _ = player.attacked(
                                        crate::random::random8() as usize,
                                        Variant::fighter().kill_name(),
                                        Some(Variant::fighter().to_key()),
                                    );
                                }
                            }
                            FighterAction::Teleport(aim) => {
                                this.take();
                                *time += start.elapsed();
                                NormalSpell::Teleport.cast(
                                    Some(arc.clone()),
                                    player,
                                    board,
                                    None,
                                    Some(aim),
                                    Some(time),
                                );
                                start = std::time::Instant::now();
                                this = Some(arc.try_write().unwrap());
                            }
                        }
                    }
                    this.as_mut().unwrap().windup -= 1;
                } else {
                    // Deciding what to do
                    if this.as_ref().unwrap().pos.is_near(player.pos, 2) {
                        // Smacking
                        this.as_mut().unwrap().variant = Variant::Fighter(FighterAction::Smack);
                        this.as_mut().unwrap().windup = 1;
                        this.as_mut().unwrap().attacking = true;
                    } else if !this.as_ref().unwrap().pos.is_near(player.pos, 10) && *line_of_sight
                    {
                        // Teleporting in
                        this.as_mut().unwrap().variant =
                            Variant::Fighter(FighterAction::Teleport(player.pos));
                        this.as_mut().unwrap().windup = 3;
                        this.as_mut().unwrap().attacking = true;
                    } else {
                        this.as_mut().unwrap().attacking = false;
                    }
                }
                *time += start.elapsed();
                true
            }
            Variant::FighterBoss { buff, action } => {
                if this.as_ref().unwrap().windup > 0 {
                    if this.as_ref().unwrap().windup == 1 {
                        // Doin' time
                        match action {
                            FighterBossAction::Teleport(aim) => {
                                this.take();
                                // TODO: change this to be an actual teleportation
                                *time += start.elapsed();
                                NormalSpell::Charge.cast(
                                    Some(arc.clone()),
                                    player,
                                    board,
                                    None,
                                    Some(aim),
                                    Some(time),
                                );
                                start = std::time::Instant::now();
                                this = Some(arc.try_write().unwrap());
                            }
                            FighterBossAction::BigExplode(aim) => {
                                this.take();
                                *time += start.elapsed();
                                NormalSpell::BigExplode.cast(
                                    Some(arc.clone()),
                                    player,
                                    board,
                                    None,
                                    Some(aim),
                                    Some(time),
                                );
                                start = std::time::Instant::now();
                                this = Some(arc.try_write().unwrap());
                            }
                            FighterBossAction::ApplyBuff => {
                                if let Variant::FighterBoss { buff, .. } =
                                    &mut this.as_mut().unwrap().variant
                                {
                                    *buff = 3;
                                } else {
                                    unreachable!("My name is Professor Bug");
                                }
                            }
                            FighterBossAction::Smack => {
                                if this.as_ref().unwrap().pos.is_near(player.pos, 2) {
                                    // Smackins
                                    let mut damage = crate::random::random8() as usize;
                                    if buff > 0 {
                                        damage *= 2;
                                    }
                                    let _ = player.attacked(
                                        damage,
                                        this.as_ref().unwrap().variant.kill_name(),
                                        Some(Variant::fighter_boss().to_key()),
                                    );
                                    if let Variant::FighterBoss { buff, .. } =
                                        &mut this.as_mut().unwrap().variant
                                    {
                                        *buff = *buff - 1;
                                    } else {
                                        unreachable!("Professor Bug, that is my name");
                                    }
                                }
                            }
                        }
                        this.as_mut().unwrap().windup = 0;
                        *time += start.elapsed();
                        true
                    } else {
                        this.as_mut().unwrap().windup -= 1;
                        *time += start.elapsed();
                        false
                    }
                } else {
                    // deciding what to do
                    this.as_mut().unwrap().attacking = true;
                    if this.as_ref().unwrap().pos.is_near(player.pos, 2) && buff > 0 {
                        // Smacking
                        this.as_mut()
                            .unwrap()
                            .variant
                            .set_fighter_boss_action(FighterBossAction::Smack);
                        this.as_mut().unwrap().windup = 1;
                    } else if this.as_ref().unwrap().pos.is_near(player.pos, 6) && buff == 0 {
                        // Teleporting away
                        let delta_x = this.as_ref().unwrap().pos.x as isize - player.pos.x as isize;
                        let delta_y = this.as_ref().unwrap().pos.y as isize - player.pos.y as isize;
                        let target_x = this.as_ref().unwrap().pos.x as isize + delta_x;
                        let target_y = this.as_ref().unwrap().pos.y as isize + delta_y;
                        let target =
                            Vector::new(target_x.max(0) as usize, target_y.max(0) as usize);
                        this.as_mut()
                            .unwrap()
                            .variant
                            .set_fighter_boss_action(FighterBossAction::Teleport(target));
                        this.as_mut().unwrap().windup = 3;
                    } else if !this.as_ref().unwrap().pos.is_near(player.pos, 10) && buff == 0 {
                        // Buffing self
                        this.as_mut()
                            .unwrap()
                            .variant
                            .set_fighter_boss_action(FighterBossAction::ApplyBuff);
                        this.as_mut().unwrap().windup = 10;
                    } else if !this.as_ref().unwrap().pos.is_near(player.pos, 10) && *line_of_sight
                    {
                        // Big explode
                        this.as_mut()
                            .unwrap()
                            .variant
                            .set_fighter_boss_action(FighterBossAction::BigExplode(player.pos));
                        this.as_mut().unwrap().windup = 9;
                    } else {
                        this.as_mut().unwrap().attacking = false;
                    }
                    *time += start.elapsed();
                    false
                }
            }
            Variant::Archer(aim) => {
                if this.as_ref().unwrap().windup > 0 {
                    if this.as_ref().unwrap().windup == 1 {
                        // Shooting time
                        let pos = this.as_ref().unwrap().pos;
                        this.take();
                        *time += start.elapsed();
                        let hit = crate::arrow(pos, aim, board, player, time);
                        start = std::time::Instant::now();
                        this = Some(arc.try_write().unwrap());
                        match hit {
                            Some(crate::Entity::Player(player)) => {
                                // if we hit a player, then deal 1 - 16 damage
                                let _ = player.attacked(
                                    (luck_roll8(player) as usize) << 1,
                                    this.as_ref().unwrap().variant.kill_name(),
                                    Some(Variant::archer().to_key()),
                                );
                            }
                            Some(crate::Entity::Enemy(arc)) => {
                                // If we hit an enemy, then deal 1 - 4 damage
                                arc.try_write().unwrap().attacked(random4() as usize);
                            }
                            None => {}
                        }
                    }
                    this.as_mut().unwrap().windup -= 1;
                    *time += start.elapsed();
                    this.as_ref().unwrap().windup == 0
                } else {
                    // Deciding
                    if *line_of_sight {
                        this.as_mut().unwrap().variant = Variant::Archer(player.pos);
                        this.as_mut().unwrap().windup = 3;
                        this.as_mut().unwrap().attacking = true;
                    } else {
                        this.as_mut().unwrap().attacking = false;
                    }
                    *time += start.elapsed();
                    false
                }
            }
            Variant::ArcherBoss(action) => {
                if this.as_ref().unwrap().windup > 0 {
                    if this.as_ref().unwrap().windup == 1 {
                        // Doing stuff
                        let pos = this.as_ref().unwrap().pos;
                        match action {
                            ArcherBossAction::Shoot(aim) => {
                                this.as_mut().unwrap().log(format!("Firing at {aim}"));
                                this.take();
                                *time += start.elapsed();
                                let hit = crate::arrow(pos, aim, board, player, time);
                                start = std::time::Instant::now();
                                this = Some(arc.try_write().unwrap());
                                if let Some(entity) = hit {
                                    match entity {
                                        crate::Entity::Player(player) => {
                                            let damage = (luck_roll8(player) << 4) as usize;
                                            this.as_mut().unwrap().log(format!(
                                                "Hit player, dealing {damage} damage"
                                            ));
                                            let _ = player.attacked(
                                                damage,
                                                this.as_ref().unwrap().variant.kill_name(),
                                                Some(Variant::archer_boss().to_key()),
                                            );
                                        }
                                        crate::Entity::Enemy(arc) => {
                                            let damage = random8() as usize;
                                            this.as_mut().unwrap().log(format!(
                                                "Hit enemy at {} for {damage}",
                                                arc.try_read().unwrap().pos
                                            ));
                                            arc.try_write().unwrap().attacked(damage);
                                        }
                                    }
                                }
                            }
                            ArcherBossAction::Barrage(mut targets) => {
                                this.as_mut()
                                    .unwrap()
                                    .log(format!("Barrage has {} targets", targets.len()));
                                if targets.len() == 5 {
                                    // we have all the targets so we can now shoot
                                    for aim in targets.into_iter() {
                                        this.as_mut().unwrap().log(format!("Firing at {aim}"));
                                        this.take();
                                        *time += start.elapsed();
                                        let hit = crate::arrow(pos, aim, board, player, time);
                                        start = std::time::Instant::now();
                                        this = Some(arc.try_write().unwrap());
                                        match hit {
                                            Some(crate::Entity::Player(player)) => {
                                                let damage = (luck_roll8(player) << 1) as usize;
                                                this.as_mut()
                                                    .unwrap()
                                                    .log(format!("Hit player for {damage}"));
                                                let _ = player.attacked(
                                                    damage,
                                                    this.as_ref().unwrap().variant.kill_name(),
                                                    Some(Variant::archer_boss().to_key()),
                                                );
                                            }
                                            Some(crate::Entity::Enemy(arc)) => {
                                                let damage = random4() as usize;
                                                this.as_mut().unwrap().log(format!(
                                                    "Hit enemy at {} for {damage}",
                                                    arc.try_read().unwrap().pos
                                                ));
                                                arc.try_write()
                                                    .unwrap()
                                                    .attacked(random4() as usize);
                                            }
                                            None => {
                                                this.as_mut().unwrap().log("Missed".to_string())
                                            }
                                        }
                                    }
                                } else if *line_of_sight {
                                    // still getting targets
                                    targets.push(player.pos);
                                    this.as_mut()
                                        .unwrap()
                                        .log(format!("Adding {} to targets", player.pos));
                                    this.as_mut().unwrap().variant =
                                        Variant::ArcherBoss(ArcherBossAction::Barrage(targets));
                                    // every other turn it adds the current position meaning 10
                                    // turns total
                                    this.as_mut().unwrap().windup = 3;
                                }
                            }
                        }
                    }
                    this.as_mut().unwrap().windup -= 1;
                    *time += start.elapsed();
                    false
                } else {
                    if *line_of_sight {
                        if random4() == 4 {
                            // Barrage
                            this.as_mut().unwrap().log("Decided to barrage".to_string());
                            this.as_mut().unwrap().variant =
                                Variant::ArcherBoss(ArcherBossAction::Barrage(Vec::new()));
                            this.as_mut().unwrap().windup = 2;
                        } else {
                            // Regular shot
                            this.as_mut()
                                .unwrap()
                                .log(format!("Decided to shoot at {}", player.pos));
                            this.as_mut().unwrap().variant =
                                Variant::ArcherBoss(ArcherBossAction::Shoot(player.pos));
                            this.as_mut().unwrap().windup = 3;
                        }
                        this.as_mut().unwrap().attacking = true;
                    } else {
                        this.as_mut().unwrap().log("Cannot see player".to_string());
                        this.as_mut().unwrap().attacking = false;
                    }
                    false
                }
            }
        }
    }
    pub fn is_near(&self, pos: Vector, range: usize) -> bool {
        self.pos.x.abs_diff(pos.x) < range && self.pos.y.abs_diff(pos.y) < range
    }
    pub fn promote(&mut self) -> Result<(), ()> {
        match self.variant {
            Variant::Basic => *self = Enemy::new(self.pos, Variant::basic_boss()),
            Variant::Mage(_) => *self = Enemy::new(self.pos, Variant::mage_boss()),
            Variant::Fighter(_) => *self = Enemy::new(self.pos, Variant::fighter_boss()),
            Variant::Archer(_) => *self = Enemy::new(self.pos, Variant::archer_boss()),
            Variant::BasicBoss(_)
            | Variant::MageBoss(_)
            | Variant::FighterBoss { .. }
            | Variant::ArcherBoss(_) => {
                return Err(());
            }
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
    fn set_reward(mut self, state: bool) -> Self {
        self.reward = state;
        self
    }
    pub fn log(&mut self, msg: String) {
        if let Some(log) = self.log.as_mut() {
            log.write_all((msg + "\n").as_bytes()).unwrap();
        }
    }
    // if it is aiming, returns whether or not it is urgent
    pub fn is_aiming(&self) -> Option<bool> {
        if self.windup != 0
            && matches!(
                self.variant,
                Variant::BasicBoss(_)
                    | Variant::Mage(MageSpell::Circle(_))
                    | Variant::MageBoss(MageBossSpell::Obamehameha(_))
                    | Variant::FighterBoss {
                        action: FighterBossAction::BigExplode(_),
                        ..
                    }
                    | Variant::Archer(_)
                    | Variant::ArcherBoss(_)
            )
        {
            Some(self.windup == 1)
        } else {
            None
        }
    }
    // Pushes aim position and whether or not it is about to fire
    pub fn get_aim_pos(&self, out: &mut Vec<(Vector, bool)>) {
        let urgent = self.windup == 1;
        match &self.variant {
            Variant::Mage(MageSpell::Circle(pos)) => out.push((*pos, urgent)),
            Variant::FighterBoss {
                action: FighterBossAction::BigExplode(pos),
                ..
            } => out.push((*pos, urgent)),
            Variant::Archer(pos) => out.push((*pos, urgent)),
            Variant::ArcherBoss(ArcherBossAction::Shoot(pos)) => out.push((*pos, urgent)),
            Variant::ArcherBoss(ArcherBossAction::Barrage(targets)) => {
                out.extend(targets.iter().map(|target| (*target, urgent)));
            }
            _ => {}
        }
    }
}
impl Clone for Enemy {
    fn clone(&self) -> Self {
        Enemy {
            health: self.health,
            variant: self.variant.clone(),
            stun: self.stun,
            windup: self.windup,
            pos: self.pos,
            active: self.active,
            attacking: self.attacking,
            dead: self.dead,
            reward: self.reward,
            log: self.log.as_ref().map(|file| file.try_clone().unwrap()),
            show_line_of_sight: self.show_line_of_sight,
            took_damage: self.took_damage,
        }
    }
}
#[derive(Clone, Debug)]
pub enum Variant {
    Basic,
    BasicBoss(Direction),
    Mage(MageSpell),
    MageBoss(MageBossSpell),
    Fighter(FighterAction), // teleport in and smack/scorpion's chain?
    FighterBoss {
        buff: usize,
        action: FighterBossAction,
    },
    Archer(Vector),
    ArcherBoss(ArcherBossAction),
}
impl Variant {
    fn detect(&self, enemy: &RwLockWriteGuard<Enemy>, board: &Board, player: &Player) -> bool {
        let cost = {
            match board.backtraces[board.x * enemy.pos.y + enemy.pos.x].cost {
                Some(cost) => cost,
                None => {
                    if enemy.pos.is_near(player.pos, 2) {
                        1
                    } else {
                        return false;
                    }
                }
            }
        };
        match self {
            Variant::Basic => advantage_pass(
                || cost <= luck_roll8(player) as usize,
                player.get_detect_mod(),
            ),
            Variant::Mage(_) => advantage_pass(
                || cost <= ((luck_roll8(player) + 1) << 2) as usize,
                player.get_detect_mod(),
            ),
            Variant::Fighter(_) => advantage_pass(
                || cost <= (luck_roll8(player) << 1) as usize,
                player.get_detect_mod(),
            ),
            Variant::Archer(_) => advantage_pass(
                || cost <= (luck_roll8(player) << 2) as usize,
                player.get_detect_mod(),
            ),
            Variant::BasicBoss(_)
            | Variant::MageBoss(_)
            | Variant::FighterBoss { .. }
            | Variant::ArcherBoss(_) => board.backtraces[board.x * enemy.pos.y + enemy.pos.x]
                .cost
                .is_some(),
        }
    }
    fn windup_color(&self) -> Color {
        // red is melee physical
        // yellow is ranged physical
        // purple is magic
        match self {
            // Non magic melee
            Variant::Basic
            | Variant::BasicBoss(_)
            | Variant::Fighter(FighterAction::Smack)
            | Variant::FighterBoss {
                action: FighterBossAction::Smack,
                ..
            } => Color::Red,

            // Magic
            Variant::Mage(_)
            | Variant::MageBoss(_)
            | Variant::Fighter(FighterAction::Teleport(_))
            | Variant::FighterBoss { .. }
            | Variant::ArcherBoss(ArcherBossAction::Barrage(_)) => Color::Purple,

            // Non magic ranged
            Variant::Archer(_) | Variant::ArcherBoss(ArcherBossAction::Shoot(_)) => Color::Yellow,
        }
    }
    fn max_health(&self) -> usize {
        match self {
            Variant::Basic => 3,
            Variant::BasicBoss(_) => 10,
            Variant::Mage(_) => 5,
            Variant::MageBoss(_) => 10,
            Variant::Fighter(_) => 5,
            Variant::FighterBoss { .. } => 15,
            Variant::Archer(_) => 3,
            Variant::ArcherBoss(_) => 3,
        }
    }
    fn parry_stun(&self) -> usize {
        match self {
            Variant::Basic => 5,
            Variant::BasicBoss(_) => 2,
            // Mage, MageBoss, Archer, and ArcherBoss, don't attack directly so they can't be stunned through a
            // parry
            Variant::Mage(_)
            | Variant::MageBoss(_)
            | Variant::Archer(_)
            | Variant::ArcherBoss(_) => {
                unimplemented!("The filthy casual actually parried it!")
            }
            Variant::Fighter(_) => 5,
            Variant::FighterBoss { .. } => 2,
        }
    }
    fn dash_stun(&self) -> usize {
        match self {
            // Strong in melee
            Variant::BasicBoss(_) | Variant::FighterBoss { .. } => 1,
            // Decent in melee
            Variant::Basic | Variant::Fighter(_) => 2,
            // Weak in melee
            Variant::Mage(_)
            | Variant::MageBoss(_)
            | Variant::Archer(_)
            | Variant::ArcherBoss(_) => 3,
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
            Variant::Fighter(_) => (10, 2),
            Variant::FighterBoss { .. } => (15, 10),
            Variant::Archer(_) => (3, 10),
            Variant::ArcherBoss(_) => (10, 5),
        }
    }
    pub fn kill_name(&self) -> &'static str {
        match self {
            Variant::Basic => "Repurposed Automata",
            Variant::BasicBoss(_) => "Specialized Automata",
            Variant::Mage(_) => "Mage Construct",
            Variant::MageBoss(_) => "Lazy Mage",
            Variant::Fighter(_) => "Squire",
            Variant::FighterBoss { .. } => "Knight",
            Variant::Archer(_) => "archer",
            Variant::ArcherBoss(_) => "archer_boss",
        }
    }
    pub fn precise_teleport(&self) -> bool {
        matches!(self, Variant::Fighter(_))
    }
    pub fn is_boss(&self) -> bool {
        matches!(
            self,
            Variant::BasicBoss(_)
                | Variant::MageBoss(_)
                | Variant::FighterBoss { .. }
                | Variant::ArcherBoss(_)
        )
    }
    // used to get which type should be promoted into the boss
    pub fn get_tier(&self) -> Result<usize, ()> {
        match self {
            Variant::Basic => Ok(1),
            Variant::Mage(_) => Ok(2),
            Variant::Archer(_) => Ok(2),
            Variant::Fighter(_) => Ok(3),
            Variant::MageBoss(_)
            | Variant::BasicBoss(_)
            | Variant::FighterBoss { .. }
            | Variant::ArcherBoss(_) => Err(()),
        }
    }
    fn mage_aggro(&self) -> bool {
        match self {
            Self::Mage(_) => true,
            // Bosses don't matter because they always have aggro
            _ => false,
        }
    }
    // returns the highest affordable variant, and how many can be bought
    // Assumes non 0 budget
    pub fn pick_variant(available: usize, optimal: bool) -> (Variant, usize) {
        let fighter = Variant::fighter().get_cost().unwrap();
        let mage = Variant::mage().get_cost().unwrap();
        let basic = Variant::basic().get_cost().unwrap();

        // At every tier there is a 1 in 8 chance for it to move to the next unoptimally, unless
        // this has been disabled

        // Fighter
        if available > fighter && (optimal || random8() != 0) {
            (Variant::fighter(), available / fighter)
        // Mage
        } else if available > mage && (optimal || random8() != 0) {
            if bool::random() {
                (Variant::mage(), available / mage)
            } else {
                (Variant::archer(), available / mage)
            }
        // Basic
        } else {
            (Variant::basic(), available / basic)
        }
    }
    pub const fn get_cost(&self) -> Option<usize> {
        Some(match self {
            Variant::Basic => 1,
            Variant::Mage(_) | Variant::Archer(_) => 5,
            Variant::Fighter(_) => 10,
            _ => return None,
        })
    }
    fn set_fighter_boss_action(&mut self, new_action: FighterBossAction) {
        if let Variant::FighterBoss { action, .. } = self {
            *action = new_action;
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
    pub const fn fighter() -> Variant {
        Variant::Fighter(FighterAction::Smack)
    }
    pub const fn fighter_boss() -> Variant {
        Variant::FighterBoss {
            buff: 0,
            action: FighterBossAction::Smack,
        }
    }
    pub const fn archer() -> Variant {
        Variant::Archer(Vector::new(0, 0))
    }
    pub const fn archer_boss() -> Variant {
        Variant::ArcherBoss(ArcherBossAction::Shoot(Vector::new(0, 0)))
    }
    pub const fn to_key(&self) -> u8 {
        match self {
            Variant::Basic => 0,
            Variant::BasicBoss(_) => 1,
            Variant::Mage(_) => 2,
            Variant::MageBoss(_) => 3,
            Variant::Fighter(_) => 4,
            Variant::FighterBoss { .. } => 5,
            Variant::Archer(_) => 6,
            Variant::ArcherBoss(_) => 7,
        }
    }
}
impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variant::Basic => write!(f, "basic"),
            Variant::Mage(_) => write!(f, "mage"),
            Variant::BasicBoss(_) => write!(f, "basic_boss"),
            Variant::MageBoss(_) => write!(f, "mage_boss"),
            Variant::Fighter(_) => write!(f, "fighter"),
            Variant::FighterBoss { .. } => write!(f, "fighter_boss"),
            Variant::Archer(_) => write!(f, "archer"),
            Variant::ArcherBoss(_) => write!(f, "archer_boss"),
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
            "fighter" => Ok(Variant::fighter()),
            "fighter_boss" => Ok(Variant::fighter_boss()),
            "archer" => Ok(Variant::archer()),
            "archer_boss" => Ok(Variant::archer_boss()),
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
    Swap,
}
#[derive(Clone, Copy, Debug)]
pub enum FighterAction {
    Teleport(Vector),
    Smack,
}
#[derive(Clone, Debug, Copy)]
pub enum FighterBossAction {
    Teleport(Vector),
    BigExplode(Vector),
    ApplyBuff,
    Smack,
}
#[derive(Clone, Debug)]
pub enum ArcherBossAction {
    Shoot(Vector),
    // Add player's position every other turn 5 times, then shoot all at once
    Barrage(Vec<Vector>),
}
pub fn luck_roll8(player: &Player) -> u8 {
    let base = crate::random() & 7;
    if player.effects.doomed.is_active() {
        return (base + 4).min(7);
    } else if player.effects.unlucky.is_active() {
        return (base + 2).min(7);
    }
    base
}
pub fn log(this: &mut Option<RwLockWriteGuard<'_, Enemy>>, msg: String) {
    this.as_mut().unwrap().log(msg)
}
