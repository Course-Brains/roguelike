use crate::{
    Board, Difficulty, Direction, Entity, FromBinary, ItemType, Style, ToBinary, Upgrades, Vector,
    commands::parse, limbs::Limbs,
};
use std::io::{Read, Write};
use std::ops::Range;
const SYMBOL: char = '@';
const STYLE: Style = *Style::new().cyan().intense(true);
#[derive(Debug)]
pub struct Player {
    pub pos: Vector,
    pub selector: Vector,
    pub health: usize,
    pub max_health: usize,
    pub energy: usize,
    pub max_energy: usize,
    pub blocking: bool,
    pub was_hit: bool,
    pub focus: Focus,
    // If the player was killed, it has the killer's name to be shown and if applicable, the
    // numerical id of killing variant, and the killing blow's damage
    pub killer: Option<(&'static str, Option<u8>, usize)>,
    pub items: Items,
    money: usize,
    pub perception: usize,
    pub effects: Effects,
    pub upgrades: Upgrades,
    // -: harder to detect, +: easier
    pub detect_mod: isize,
    pub aiming: bool,
    // Whether or not the selector should move faster
    pub fast: bool,
    pub limbs: Limbs,
    // The memorized position
    pub memory: Option<Vector>,
}
impl Player {
    pub fn new(pos: Vector) -> Player {
        Player {
            pos,
            selector: pos,
            health: Player::starting_health(),
            max_health: Player::starting_max_health(),
            energy: Player::starting_energy(),
            max_energy: Player::starting_max_energy(),
            blocking: false,
            was_hit: false,
            focus: Focus::Player,
            killer: None,
            items: Player::starting_items(),
            money: Player::starting_money(),
            perception: Player::starting_perception(),
            effects: Effects::new(),
            upgrades: crate::Upgrades::new(),
            detect_mod: 0,
            aiming: false,
            fast: false,
            limbs: Limbs::new(),
            memory: None,
        }
    }
    fn starting_max_health() -> usize {
        match crate::SETTINGS.difficulty() {
            Difficulty::Normal => 50,
            Difficulty::Easy => 100,
            Difficulty::Hard => 25,
        }
    }
    fn starting_health() -> usize {
        match crate::SETTINGS.difficulty() {
            Difficulty::Normal => 20,
            Difficulty::Easy => 100,
            Difficulty::Hard => 1,
        }
    }
    fn starting_max_energy() -> usize {
        match crate::SETTINGS.difficulty() {
            Difficulty::Normal => 3,
            Difficulty::Easy => 6,
            Difficulty::Hard => 1,
        }
    }
    fn starting_energy() -> usize {
        match crate::SETTINGS.difficulty() {
            Difficulty::Normal => 2,
            Difficulty::Easy => 6,
            Difficulty::Hard => 0,
        }
    }
    fn starting_money() -> usize {
        if crate::SETTINGS.difficulty() < crate::Difficulty::Normal {
            50
        } else {
            0
        }
    }
    fn starting_perception() -> usize {
        match crate::SETTINGS.difficulty() {
            Difficulty::Normal => 10,
            Difficulty::Easy => 20,
            Difficulty::Hard => 0,
        }
    }
    fn starting_items() -> Items {
        if crate::SETTINGS.difficulty() == Difficulty::Hard {
            [Some(ItemType::FarSightPotion), None, None, None, None, None]
        } else {
            [None; 6]
        }
    }
    pub fn do_move(&mut self, direction: Direction, board: &mut Board) {
        crate::log!("Moving from {} in {direction}", self.pos);
        self.pos += direction;
        if let Some(piece) = &board[self.pos] {
            crate::log!("  Triggering on_step at {}", self.pos);
            if piece.on_step(Entity::Player(self)) {
                crate::log!("    Removing piece");
                board[self.pos] = None;
            }
        }
        if let Some((circle, index)) = board.contact_spell_at(self.pos) {
            crate::log!("  Triggering spell at {}", self.pos);
            if let Some(caster) = &circle.caster {
                circle
                    .spell
                    .unwrap_contact()
                    .cast(Entity::Player(self), Entity::Enemy(caster.clone()));
            }
            board.spells.swap_remove(index);
        }
    }
    // Returns whether the attack was successful(Ok) and whether the player died
    // true: died
    // false: alive
    pub fn attacked(
        &mut self,
        mut damage: usize,
        attacker: &'static str,
        variant_id: Option<u8>,
    ) -> Result<bool, ()> {
        // Damage nullification
        if self.effects.invincible.is_active() {
            crate::stats().damage_invulned += damage;
            return Err(());
        }
        self.was_hit = true;
        if self.blocking {
            crate::stats().damage_blocked += damage;
            return Err(());
        }
        // Half damage taken on easy
        if crate::SETTINGS.difficulty() == crate::Difficulty::Easy && damage > 1 {
            damage /= 2;
        }
        // If no damage was taken then there is no need to punish the player for taking damage
        if damage == 0 {
            return Ok(false);
        }

        // By this point, it has beeen determined that the player is taking damage.

        // Notifying the player that they took damage
        crate::bell(Some(&mut std::io::stdout()));

        // One shot protection
        if self.health == self.max_health && damage > self.health {
            crate::log!(
                "Player was hit for {damage}, but one shot protection is reducing to {}",
                self.health - 1
            );
            damage = self.health - 1;
        }
        // Damage has been determined
        if crate::BONUS_NO_DAMAGE.load(crate::RELAXED) {
            crate::set_feedback("Couldn't you have avoided that hit?".to_string());
            crate::bell(Some(&mut std::io::stdout()));
        }
        crate::BONUS_NO_DAMAGE.store(false, crate::RELAXED);
        crate::stats().hits_taken += 1;

        if self.should_remove_limb(damage) {
            self.limbs.remove_random_limb();
        }

        if self.health <= damage {
            self.killer = Some((attacker, variant_id, damage));
            crate::stats().damage_taken += self.health;
            return Ok(true);
        }
        crate::stats().damage_taken += damage;
        self.health -= damage;
        Ok(false)
    }
    pub fn should_remove_limb(&self, damage: usize) -> bool {
        crate::log!("Deciding if a limb should be lost:");
        let health = self.health as f64;
        let max_health = self.max_health as f64;
        let energy = self.energy as f64;
        let max_energy = self.max_energy as f64;
        let damage = damage as f64;

        let health_weight =
            ((max_health - health) / max_health) * crate::limbs::LIMB_LOSS_HEALTH_WEIGHT;
        crate::log!("  health weight: {health_weight}");
        let energy_weight =
            ((max_energy - energy) / max_energy) * crate::limbs::LIMB_LOSS_ENERGY_WEIGHT;
        crate::log!("  energy_weight: {energy_weight}");
        let damage_weight = (damage / max_health) * crate::limbs::LIMB_LOSS_DAMAGE_WEIGHT;
        crate::log!("  damage weight: {damage_weight}");

        let mut pass = (health_weight + energy_weight + damage_weight) as crate::Rand;
        crate::log!("  pass value: {pass}");

        if crate::SETTINGS.difficulty() == crate::Difficulty::Easy {
            pass /= 2;
            crate::log!("Halving pass due to difficulty, now at: {pass}")
        } else if crate::SETTINGS.difficulty() >= crate::Difficulty::Hard {
            pass *= 2;
            crate::log!("Doubling pass due to difficulty, now at {pass}");
        }

        pass > crate::random()
    }
    pub fn on_kill(&mut self, enemy: &crate::Enemy) {
        crate::stats().add_kill(enemy.variant.clone());
        if !enemy.reward {
            return;
        }
        let (energy, health) = enemy.variant.kill_value();
        if self.upgrades.lifesteal {
            self.health += energy;
        }
        for _ in 0..energy {
            if self.energy < self.max_energy {
                self.energy += 1;
            } else if self.health < self.max_health {
                crate::stats().damage_healed += 1;
                self.health += health;
            } else {
                if crate::BONUS_NO_WASTE.load(crate::RELAXED) {
                    crate::set_feedback("So wasteful...".to_string());
                    crate::bell(Some(&mut std::io::stdout()));
                }
                crate::BONUS_NO_WASTE.store(false, crate::RELAXED);
                crate::stats().energy_wasted += 1;
            }
        }
        self.health = self.health.min(self.max_health);
        crate::log!(
            "Killed {}, health is now: {}, energy is now: {}",
            enemy.variant,
            self.health,
            self.energy
        );
        if self.upgrades.full_energy_ding
            && self.energy == self.max_energy
            && (self.health == self.max_health || self.upgrades.lifesteal)
        {
            crate::bell(Some(&mut std::io::stdout()));
        }
    }
    pub fn get_focus(&self) -> Vector {
        match self.focus {
            Focus::Player => self.pos,
            Focus::Selector => self.selector,
        }
    }
    pub fn is_dead(&self) -> bool {
        self.killer.is_some()
    }
    // returns whether or not they want to see their stats
    // By this point, death stat collection has happened
    pub fn handle_death(state: &crate::State) -> bool {
        crate::log!("Handling death of player");
        let killer = state.player.killer.unwrap();
        println!(
            "\x1b[2J\x1b[15;0HYou were killed by {}{}\x1b[0m.",
            Style::new().green().intense(true),
            killer.0
        );
        Player::death_message(state);
        print!(
            "\nPress {}Enter\x1b[0m to exit. Or press {}S\x1b[0m to see stats.",
            Style::new().cyan(),
            Style::new().cyan()
        );
        std::io::stdout().flush().unwrap();
        let mut buf = [0];
        let mut stdin = std::io::stdin().lock();
        loop {
            stdin.read_exact(&mut buf).unwrap();
            match buf[0].to_ascii_uppercase() {
                b'S' => break true,
                b'\n' => break false,
                _ => {}
            }
        }
    }
    pub fn death_message(state: &crate::State) {
        crate::log!("Getting death message");
        let mut out = std::io::stdout().lock();
        if state.level == 0
            && state.turn < 300
            && std::fs::File::open("stats")
                .is_ok_and(|file| file.metadata().is_ok_and(|meta| meta.len() > 10000))
        {
            write!(out, "Just roll better next time, lmao.").unwrap();
            return;
        }
        if crate::stats().cowardice > state.level / 3 && crate::random() & 3 == 0 {
            write!(out, "Coward.").unwrap();
            return;
        }
        crate::log!("Decided not to do special death message");

        let mut kills = 0;
        for kill in crate::stats().kills.values() {
            kills += kill
        }

        let mut stat_len = None;
        if let Ok(file) = std::fs::File::open("stats") {
            stat_len = Some(file.metadata().unwrap().len());
        }

        let stats = crate::stats();

        crate::log!("Doing normal death message");

        let mut valid = vec![
            "Do better next time.",
            "You CAN prevail.",
            "Bad luck.",
            "Did you know? You died. Now you know.",
            "With enough luck you'll win eventually, even without skill.",
            "Try, try, and try again.",
            "Better luck next time!",
        ];
        // If it is not on easy mode
        if crate::SETTINGS.difficulty() != crate::Difficulty::Easy {
            valid.push("You should try easy mode.");
        }
        // If the player killed themself
        if state.player.killer.unwrap().1.is_none() {
            valid.push("If you kill yourself that well then we'll be out of a job.")
        }
        // If the player was killed by a basic
        else if state.player.killer.unwrap().1.unwrap() == crate::enemy::Variant::basic().to_key()
        {
            valid.push(
                "You do realize how many things were waiting to try and kill you right?\n \
                Think of how disapointed they'll be when they find out you were done in by\n \
                the absolute weakest of the lot.",
            )
        }
        // If the player is a coward but died to a boss anyway
        else if crate::enemy::Variant::from_key(state.player.killer.unwrap().1.unwrap()).is_boss()
            && stats.cowardice > state.level
        {
            valid.push("Maybe you should have run from that one, like you did with the others.");
        }
        // If they died to a basic_boss on level 0
        if state.level == 0
            && state
                .player
                .killer
                .unwrap()
                .1
                .is_some_and(|key| key == crate::enemy::Variant::basic_boss().to_key())
        {
            valid.push("Fair enough, that thing is strong.")
        }
        // If the stats file is more than 500 bytes
        if let Ok(stats) = std::fs::File::open("stats")
            && stats.metadata().unwrap().len() > 500
        {
            valid.push("Have you heard the definition of insanity?")
        }
        // If they have more doors opened than kills
        if stats.doors_opened / 10 > kills {
            valid.push(
                "Maybe if you spent more time actually fighting instead of opening doors \
                you'd still be alive.",
            )
        }
        // If they haven't disabled kicking doors and the stat file is below a certain length and
        // less than 1% of the doors opened we done so by kicking them
        if crate::SETTINGS.kick_doors()
            && stat_len.is_none_or(|len| len < 500)
            && stats.doors_opened_by_walking < stats.doors_opened / 100
        {
            valid.push("You can open doors by walking into them.")
        }
        crate::log!("Valid death messages {valid:?}");
        let index = crate::random::random_index(valid.len()).unwrap();
        crate::log!("Picked {}", valid[index]);

        write!(out, "{}", valid[index]).unwrap();
        crate::log!("Done printing death message");
    }
    // returns whether or not the item was added successfully
    pub fn add_item(&mut self, item: ItemType) -> bool {
        crate::log!("Adding {item} to player");
        let mut lock = std::io::stdin().lock();
        let mut buf = [0];
        Board::set_desc(
            &mut std::io::stdout(),
            "Select slot for the item(1-6) or c to cancel",
        );
        std::io::stdout().flush().unwrap();
        let selected = loop {
            lock.read_exact(&mut buf).unwrap();
            crate::log!("  recieved {}", buf[0].to_string());
            match buf[0] {
                b'1' => break Some(0),
                b'2' => break Some(1),
                b'3' => break Some(2),
                b'4' => break Some(3),
                b'5' => break Some(4),
                b'6' => break Some(5),
                b'c' => break None,
                _ => continue,
            }
        };
        match selected {
            Some(index) => {
                crate::log!("  Putting item in slot {index}");
                self.items[index] = Some(item);
                crate::stats().add_item(item);
                true
            }
            None => {
                crate::log!("  Pickup canceled");
                false
            }
        }
    }
    pub fn decriment_effects(&mut self) {
        self.effects.decriment()
    }
    pub fn heal(&mut self, amount: usize) {
        self.health += amount;
        crate::stats().damage_healed += amount;
        if self.health > self.max_health {
            crate::stats().damage_healed -= self.health - self.max_health;
            self.health = self.max_health;
        }
    }
    pub fn heal_to_full(&mut self) {
        self.heal(self.max_health - self.health)
    }
    pub fn aim(&mut self, board: &mut Board) {
        let mut specials = Vec::new();
        for pos in crate::ray_cast(self.pos, self.selector, board, None, true, self.pos)
            .0
            .iter()
        {
            specials.push(board.add_special(crate::board::Special::new(
                *pos,
                ' ',
                Some(*Style::new().background_green()),
            )));
        }
        board.smart_render(self);
        std::mem::drop(specials);
    }
    pub fn give_money(&mut self, amount: usize) {
        self.money += amount;
        crate::stats().total_money += amount;
    }
    pub fn have_money(&mut self, amount: usize) -> bool {
        self.money >= amount
    }
    pub fn take_money(&mut self, amount: usize) {
        self.money -= amount;
    }
    pub fn get_money(&self) -> usize {
        self.money
    }
    // not actually unsafe in the sense of undefined behavior, but it does bypass all the stat
    // saving mechanisms so you shouldn't use this unless you actually know that it doesn't cause
    // problems
    pub unsafe fn mut_money(&mut self) -> &mut usize {
        &mut self.money
    }
    // gets the damage dealt by the player per hit
    pub fn get_damage(&self) -> usize {
        let mut damage = 1;
        if self.effects.damage_boost.is_active() {
            damage += 1
        }
        if self.effects.drunk.is_active() {
            damage += 1
        }
        if crate::SETTINGS.difficulty() == crate::Difficulty::Easy {
            damage += 1
        }
        damage
    }
    pub fn stat_choice(&mut self) {
        crate::log!("Granting stats");
        crate::set_desc("1: more health, 2: more energy, 3: more perception");
        let mut buf = [0];
        let mut lock = std::io::stdin().lock();
        let easy = crate::SETTINGS.difficulty() == crate::Difficulty::Easy;
        loop {
            lock.read_exact(&mut buf).unwrap();
            match buf[0] {
                b'1' => {
                    crate::log!("  Chosen health");
                    if easy {
                        self.max_health += (self.max_health / 2).max(1);
                        self.heal_to_full();
                    } else {
                        self.max_health += (self.max_health / 5).max(1)
                    }
                }
                b'2' => {
                    crate::log!("  Chosen energy");
                    if easy {
                        self.max_energy += (self.max_energy / 2).max(1);
                    }
                    self.max_energy += (self.max_energy / 5).max(1)
                }
                b'3' => {
                    crate::log!("  Chosen perception");
                    if easy {
                        self.perception += (self.perception / 2).max(1);
                    } else {
                        self.perception += (self.perception / 5).max(1);
                    }
                }
                other => {
                    crate::log!("  Recieved \"{}\", trying again", char::from(other));
                    continue;
                }
            }
            break;
        }
    }
    pub fn get_perception(&self) -> usize {
        let mut perception = self.perception;
        let eyes = self.limbs.count_eyes();
        if eyes == 1 {
            perception /= 2;
        } else if eyes == 0 {
            perception = 0;
        }
        perception += self.limbs.count_normal_eyes() * 5;
        if self.effects.drunk.is_active() {
            perception /= 2
        }
        if self.effects.far_sight.is_active() {
            perception += 10;
            perception *= 2;
        }
        perception
    }
    pub fn get_detect_mod(&self) -> isize {
        if self.effects.drunk.is_active() {
            self.detect_mod + 1
        } else {
            self.detect_mod
        }
    }
}
// Rendering
impl Player {
    pub fn draw(&self, board: &Board, bounds: Range<Vector>) {
        let mut lock = std::io::stdout().lock();
        self.draw_player(&mut lock, bounds, board);
        self.draw_health(board, &mut lock);
        self.draw_energy(board, &mut lock);
        self.draw_items(board, &mut lock);
        self.draw_limbs(board, &mut lock);
    }
    fn draw_player(&self, lock: &mut impl std::io::Write, bounds: Range<Vector>, board: &Board) {
        if !bounds.contains(&self.pos) {
            return;
        }
        let is_aiming = board.is_enemy_aiming();
        let style = if self.limbs.count_seer_eyes() == 1
            && let Some(is_aiming) = is_aiming
        {
            *STYLE.clone().background_red().intense_background(is_aiming)
        } else {
            STYLE
        };
        crossterm::queue!(lock, (self.pos - bounds.start).to_move()).unwrap();
        write!(lock, "{style}{SYMBOL}\x1b[0m").unwrap();
    }
    fn draw_health(&self, board: &Board, lock: &mut impl std::io::Write) {
        crossterm::queue!(
            lock,
            crossterm::cursor::MoveTo(1, (board.render_y * 2) as u16 + 1)
        )
        .unwrap();
        let split = (self.health * 50) / self.max_health;
        write!(
            lock,
            "\x1b[2K[\x1b[32m{}\x1b[31m{}\x1b[0m] {}/{}",
            "#".repeat(split),
            "-".repeat(50 - split),
            self.health,
            self.max_health,
        )
        .unwrap();
    }
    fn draw_energy(&self, board: &Board, lock: &mut impl std::io::Write) {
        crossterm::queue!(
            lock,
            crossterm::cursor::MoveTo(1, (board.render_y * 2) as u16 + 2)
        )
        .unwrap();
        let split = (self.energy * 50) / self.max_energy;
        write!(
            lock,
            "\x1b[2K[\x1b[96m{}\x1b[0m{}] {}/{}",
            "#".repeat(split),
            "-".repeat(50 - split),
            self.energy,
            self.max_energy
        )
        .unwrap();
    }
    fn draw_items(&self, board: &Board, lock: &mut impl std::io::Write) {
        // Maximum initial y is 6 * 3 = 18, but accounting for the last item, the last allocated y
        // is 21
        for (index, item) in self.items.iter().enumerate() {
            if let Some(item) = item {
                crossterm::queue!(
                    lock,
                    Vector::new(board.render_x * 2 + 2, index * 3).to_move(),
                    crossterm::cursor::SavePosition
                )
                .unwrap();
                item.name(lock);
            }
        }
    }
    fn draw_limbs(&self, board: &Board, lock: &mut impl std::io::Write) {
        // 1 per line, starting at line 23
        let start = Vector::new(board.render_x * 2 + 2, 23);
        self.limbs.draw(start, lock);
    }
    pub fn reposition_cursor(&mut self, underscore: bool, bounds: Range<Vector>) {
        self.selector = self
            .selector
            .clamp(bounds.start..bounds.end - Vector::new(1, 1));
        crossterm::execute!(std::io::stdout(), (self.selector - bounds.start).to_move()).unwrap();
        if underscore {
            crossterm::execute!(
                std::io::stdout(),
                crossterm::cursor::SetCursorStyle::SteadyUnderScore
            )
            .unwrap()
        } else {
            crossterm::execute!(
                std::io::stdout(),
                crossterm::cursor::SetCursorStyle::DefaultUserShape
            )
            .unwrap()
        }
        std::io::stdout().flush().unwrap();
    }
}
// TODO: Find a better way to do this, I couldn't figure out how to do ::methods() on an array, but
// this works I guess?
type Items = [Option<ItemType>; 6];
impl FromBinary for Player {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Player {
            pos: Vector::from_binary(binary)?,
            selector: Vector::from_binary(binary)?,
            health: usize::from_binary(binary)?,
            max_health: usize::from_binary(binary)?,
            energy: usize::from_binary(binary)?,
            max_energy: usize::from_binary(binary)?,
            blocking: bool::from_binary(binary)?,
            was_hit: bool::from_binary(binary)?,
            focus: Focus::from_binary(binary)?,
            // the player has to be alive to save
            killer: None,
            items: Items::from_binary(binary)?,
            money: usize::from_binary(binary)?,
            perception: usize::from_binary(binary)?,
            effects: Effects::from_binary(binary)?,
            upgrades: Upgrades::from_binary(binary)?,
            detect_mod: isize::from_binary(binary)?,
            aiming: bool::from_binary(binary)?,
            fast: bool::from_binary(binary)?,
            limbs: Limbs::from_binary(binary)?,
            memory: Option::from_binary(binary)?,
        })
    }
}
impl ToBinary for Player {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.pos.to_binary(binary)?;
        self.selector.to_binary(binary)?;
        self.health.to_binary(binary)?;
        self.max_health.to_binary(binary)?;
        self.energy.to_binary(binary)?;
        self.max_energy.to_binary(binary)?;
        self.blocking.to_binary(binary)?;
        self.was_hit.to_binary(binary)?;
        self.focus.to_binary(binary)?;
        // skipping killer
        self.items
            .each_ref()
            .map(|x| x.as_ref())
            .to_binary(binary)?;
        self.money.to_binary(binary)?;
        self.perception.to_binary(binary)?;
        self.effects.to_binary(binary)?;
        self.upgrades.to_binary(binary)?;
        self.detect_mod.to_binary(binary)?;
        self.aiming.to_binary(binary)?;
        self.fast.to_binary(binary)?;
        self.limbs.to_binary(binary)?;
        self.memory.as_ref().to_binary(binary)
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Focus {
    Player,
    Selector,
}
impl Focus {
    pub fn cycle(&mut self) {
        match self {
            Focus::Player => *self = Focus::Selector,
            Focus::Selector => *self = Focus::Player,
        }
    }
}
impl FromBinary for Focus {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match bool::from_binary(binary)? {
            true => Focus::Player,
            false => Focus::Selector,
        })
    }
}
impl ToBinary for Focus {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        match self {
            Focus::Player => true,
            Focus::Selector => false,
        }
        .to_binary(binary)
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Effects {
    // self explanitory
    pub invincible: Duration,
    // Heal 2 health per turn
    pub regen: Duration,
    // make enemies roll better(+2 on 1-8)
    pub unlucky: Duration,
    // make enemies roll even better(+4)
    pub doomed: Duration,
    // +1 to damage
    pub damage_boost: Duration,
    // debug full visibility
    pub full_vis: Duration,
    // Lower perception, more detectable, increased damage
    pub drunk: Duration,
    // perception +10 then *2
    pub far_sight: Duration,
    // 1 damage per turn
    pub poison: Duration,
}
impl Effects {
    // Creates an instance with starting effects
    fn new() -> Effects {
        Effects {
            invincible: Duration::None,
            regen: Duration::None,
            unlucky: match crate::SETTINGS.difficulty() >= crate::Difficulty::Hard {
                true => Duration::Infinite,
                false => Duration::None,
            },
            doomed: Duration::None,
            damage_boost: Duration::None,
            full_vis: Duration::None,
            drunk: Duration::None,
            far_sight: Duration::None,
            poison: Duration::None,
        }
    }
    // Decreases all effect durations by 1 turn
    fn decriment(&mut self) {
        self.invincible.decriment();
        self.regen.decriment();
        self.unlucky.decriment();
        self.doomed.decriment();
        self.damage_boost.decriment();
        self.full_vis.decriment();
        self.drunk.decriment();
        self.far_sight.decriment();
        self.poison.decriment();
    }
    // for setting effects by command
    pub fn set(&mut self, s: &str) -> Result<(), String> {
        let mut split = s.split(' ');
        match split.next() {
            Some(effect) => {
                let args: String = split.map(|s| s.to_string() + " ").collect();
                match effect {
                    "invincible" => self.invincible = args.parse()?,
                    "regen" => self.regen = args.parse()?,
                    "unlucky" => self.unlucky = args.parse()?,
                    "doomed" => self.doomed = args.parse()?,
                    "damage_boost" => self.damage_boost = args.parse()?,
                    "full_vis" => self.full_vis = args.parse()?,
                    "drunk" => self.drunk = args.parse()?,
                    "far_sight" => self.far_sight = args.parse()?,
                    "poison" => self.poison = args.parse()?,
                    other => return Err(format!("{other} is not an effect")),
                }
            }
            None => return Err("No effect specified".to_string()),
        }
        Ok(())
    }
    pub fn list(&self) {
        if self.invincible.is_active() {
            println!("    and is invincible for ");
            self.invincible.list();
        }
        if self.regen.is_active() {
            println!("    and is regenerating for ");
            self.regen.list();
        }
        if self.unlucky.is_active() {
            println!("    and is unlucky for ");
            self.unlucky.list();
        }
        if self.doomed.is_active() {
            println!("    and is doomed for");
            self.doomed.list()
        }
        if self.damage_boost.is_active() {
            println!("    and has boosted damage for");
            self.damage_boost.list()
        }
        if self.full_vis.is_active() {
            println!("    and can see everything for");
            self.full_vis.list();
        }
        if self.drunk.is_active() {
            println!("    and is drunk for");
            self.drunk.list();
        }
        if self.far_sight.is_active() {
            println!("    and has far sight for");
            self.far_sight.list()
        }
        if self.poison.is_active() {
            println!("    and is poisoned for");
            self.poison.list()
        }
    }
    pub fn has_none(&self) -> bool {
        !(self.invincible.is_active()
            || self.regen.is_active()
            || self.unlucky.is_active()
            || self.doomed.is_active()
            || self.damage_boost.is_active()
            || self.full_vis.is_active()
            || self.drunk.is_active()
            || self.far_sight.is_active()
            || self.poison.is_active())
    }
}
impl FromBinary for Effects {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Effects {
            invincible: Duration::from_binary(binary)?,
            regen: Duration::from_binary(binary)?,
            unlucky: Duration::from_binary(binary)?,
            doomed: Duration::from_binary(binary)?,
            damage_boost: Duration::from_binary(binary)?,
            full_vis: Duration::from_binary(binary)?,
            drunk: Duration::from_binary(binary)?,
            far_sight: Duration::from_binary(binary)?,
            poison: Duration::from_binary(binary)?,
        })
    }
}
impl ToBinary for Effects {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.invincible.to_binary(binary)?;
        self.regen.to_binary(binary)?;
        self.unlucky.to_binary(binary)?;
        self.doomed.to_binary(binary)?;
        self.damage_boost.to_binary(binary)?;
        self.full_vis.to_binary(binary)?;
        self.drunk.to_binary(binary)?;
        self.far_sight.to_binary(binary)?;
        self.poison.to_binary(binary)
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Duration {
    None,
    // Stops just before hitting 0, so to do 10 turns, set to 11
    Turns(usize),
    Infinite,
}
impl Duration {
    fn decriment(&mut self) {
        match self {
            Self::None => {}
            Self::Turns(turns) => {
                *turns -= 1;
                if *turns == 0 {
                    *self = Self::None
                }
            }
            Self::Infinite => {}
        }
    }
    pub fn is_active(self) -> bool {
        match self {
            Self::None => false,
            Self::Turns(_) => true,
            Self::Infinite => true,
        }
    }
    pub fn remove(&mut self) {
        *self = Duration::None;
    }
    pub fn increase_to(&mut self, increment: usize, max: usize) {
        match self {
            Self::None => *self = Self::Turns(increment),
            Self::Turns(current) => {
                if *current > max {
                    return;
                }
                *current += increment;
                if *current > max {
                    *current = max
                }
            }
            Self::Infinite => {}
        }
    }
    fn list(&self) {
        match self {
            Self::None => unreachable!(),
            Self::Turns(turns) => print!("{turns} turns"),
            Self::Infinite => print!("forever"),
        }
    }
}
impl std::ops::AddAssign<usize> for Duration {
    fn add_assign(&mut self, rhs: usize) {
        match self {
            Self::None => *self = Self::Turns(rhs),
            Self::Turns(turns) => *turns += rhs,
            Self::Infinite => {}
        }
    }
}
impl std::ops::MulAssign<usize> for Duration {
    fn mul_assign(&mut self, rhs: usize) {
        if let Self::Turns(turns) = self {
            *turns *= rhs;
        }
    }
}
impl std::str::FromStr for Duration {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(' ');
        match split.next().map(|s| s.trim()) {
            Some("none") => Ok(Duration::None),
            Some("turns") => Ok(Duration::Turns(parse(split.next())?)),
            Some("infinite") => Ok(Duration::Infinite),
            Some("") => Ok(Duration::Infinite),
            Some(other) => Err(format!("{other} is not a valid duration")),
            None => Err("Did not get duration".to_string()),
        }
    }
}
impl FromBinary for Duration {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Duration::None,
            1 => Duration::Turns(usize::from_binary(binary)?),
            2 => Duration::Infinite,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not get Duration from binary",
                ));
            }
        })
    }
}
impl ToBinary for Duration {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        match self {
            Duration::None => 0_u8.to_binary(binary),
            Duration::Turns(turns) => {
                1_u8.to_binary(binary)?;
                turns.to_binary(binary)
            }
            Duration::Infinite => 2_u8.to_binary(binary),
        }
    }
}
