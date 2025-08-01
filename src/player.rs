use crate::{
    Board, Direction, Entity, FromBinary, ItemType, Style, ToBinary, Upgrades, Vector,
    commands::parse,
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
    killer: Option<&'static str>,
    pub items: Items,
    money: usize,
    pub perception: usize,
    pub effects: Effects,
    pub upgrades: Upgrades,
    // -: harder to detect, +: easier
    pub detect_mod: isize,
    // Gives you info on what you are hovering over
    pub inspect: bool,
    pub aiming: bool,
    // Whether or not the selector should move faster
    pub fast: bool,
}
impl Player {
    pub fn new(pos: Vector) -> Player {
        Player {
            pos,
            selector: pos,
            health: 20,
            max_health: 50,
            energy: 2,
            max_energy: 3,
            blocking: false,
            was_hit: false,
            focus: Focus::Player,
            killer: None,
            items: [None; 6],
            money: 0,
            perception: 10,
            effects: Effects::new(),
            upgrades: crate::Upgrades::new(),
            detect_mod: 0,
            inspect: true,
            aiming: false,
            fast: false,
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
    pub fn attacked(&mut self, damage: usize, attacker: &'static str) -> Result<bool, ()> {
        if self.effects.invincible.is_active() {
            crate::stats().damage_invulned += damage;
            return Err(());
        }
        self.was_hit = true;
        if self.blocking {
            crate::stats().damage_blocked += damage;
            return Err(());
        }
        if self.health <= damage {
            self.killer = Some(attacker);
            crate::stats().damage_taken += self.health;
            return Ok(true);
        }
        crate::stats().damage_taken += damage;
        self.health -= damage;
        Ok(false)
    }
    pub fn on_kill(&mut self, enemy: &crate::Enemy) {
        crate::stats().kills += 1;
        if !enemy.reward {
            return;
        }
        let (energy, health) = enemy.variant.kill_value();
        for _ in 0..energy {
            if self.energy < self.max_energy {
                self.energy += 1;
            } else if self.health < self.max_health {
                self.health += health;
            } else {
                break;
            }
        }
        self.health = self.health.min(self.max_health);
        crate::log!(
            "Killed {}, health is now: {}, energy is now: {}",
            enemy.variant,
            self.health,
            self.energy
        );
    }
    pub fn get_focus(&self) -> Vector {
        match self.focus {
            Focus::Player => self.pos,
            Focus::Selector => self.selector,
        }
    }
    // returns whether or not the player is dead
    pub fn handle_death(&self) -> bool {
        match self.killer {
            Some(killer) => {
                println!(
                    "\x1b[2J\x1b[15;0HYou were killed by {}{}\x1b[0m.",
                    Style::new().green().intense(true),
                    killer
                );
                Player::death_message();
                print!("\nPress {}Enter\x1b[0m to exit.", Style::new().cyan());
                std::io::stdout().flush().unwrap();
                loop {
                    if let crate::input::Input::Enter = crate::input::Input::get() {
                        break;
                    }
                }
                true
            }
            None => false,
        }
    }
    pub fn death_message() {
        let mut out = std::io::stdout().lock();
        match crate::random() % 4 {
            0 => write!(out, "Do better next time."),
            1 => write!(
                out,
                "With enough luck you'll eventually with even without skill."
            ),
            2 => write!(out, "You CAN prevail."),
            3 => write!(out, "Have you ever heard of the definition of insanity?"),
            _ => unreachable!("Fuckity wuckity someone is bad at math"),
        }
        .unwrap();
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
    pub unsafe fn mut_money(&mut self) -> &mut usize {
        &mut self.money
    }
}
// Rendering
impl Player {
    pub fn draw(&self, board: &Board, bounds: Range<Vector>) {
        let mut lock = std::io::stdout().lock();
        self.draw_player(&mut lock, bounds);
        self.draw_health(board, &mut lock);
        self.draw_energy(board, &mut lock);
        self.draw_items(board, &mut lock);
    }
    fn draw_player(&self, lock: &mut impl std::io::Write, bounds: Range<Vector>) {
        if !bounds.contains(&self.pos) {
            return;
        }
        crossterm::queue!(lock, (self.pos - bounds.start).to_move()).unwrap();
        write!(lock, "{STYLE}{SYMBOL}\x1b[0m").unwrap();
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
        for (index, item) in self.items.iter().enumerate() {
            if let Some(item) = item {
                crossterm::queue!(
                    lock,
                    Vector::new(board.render_x * 2 + 2, index * 5).to_move(),
                    crossterm::cursor::SavePosition
                )
                .unwrap();
                item.name(lock);
            }
        }
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
            inspect: bool::from_binary(binary)?,
            aiming: bool::from_binary(binary)?,
            fast: bool::from_binary(binary)?,
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
        self.inspect.to_binary(binary)?;
        self.aiming.to_binary(binary)?;
        self.fast.to_binary(binary)
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
    // No perception check on enemies, but aggro all mage types
    pub mage_sight: Duration,
    // Heal 2 health per turn
    pub regen: Duration,
    // make enemies roll better(+2 on 1-8)
    pub unlucky: Duration,
    // make enemies roll even better(+4)
    pub doomed: Duration,
}
impl Effects {
    // Creates an instance with no effects
    fn new() -> Effects {
        Effects {
            invincible: Duration::None,
            mage_sight: Duration::None,
            regen: Duration::None,
            unlucky: Duration::None,
            doomed: Duration::None,
        }
    }
    // Decreases all effect durations by 1 turn
    fn decriment(&mut self) {
        self.invincible.decriment();
        self.mage_sight.decriment();
        self.regen.decriment();
        self.unlucky.decriment();
        self.doomed.decriment();
    }
    // for setting effects by command
    pub fn set(&mut self, s: &str) -> Result<(), String> {
        let mut split = s.split(' ');
        match split.next() {
            Some(effect) => {
                let args: String = split.map(|s| s.to_string() + " ").collect();
                match effect {
                    "invincible" => self.invincible = args.parse()?,
                    "mage_sight" => self.mage_sight = args.parse()?,
                    "regen" => self.regen = args.parse()?,
                    "unlucky" => self.unlucky = args.parse()?,
                    "doomed" => self.doomed = args.parse()?,
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
        if self.mage_sight.is_active() {
            println!("    and has mage sight for ");
            self.mage_sight.list();
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
    }
    pub fn has_none(&self) -> bool {
        if self.invincible.is_active() {
            return false;
        }
        if self.mage_sight.is_active() {
            return false;
        }
        if self.regen.is_active() {
            return false;
        }
        if self.unlucky.is_active() {
            return false;
        }
        if self.doomed.is_active() {
            return false;
        }
        true
    }
}
impl FromBinary for Effects {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Effects {
            invincible: Duration::from_binary(binary)?,
            mage_sight: Duration::from_binary(binary)?,
            regen: Duration::from_binary(binary)?,
            unlucky: Duration::from_binary(binary)?,
            doomed: Duration::from_binary(binary)?,
        })
    }
}
impl ToBinary for Effects {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.invincible.to_binary(binary)?;
        self.mage_sight.to_binary(binary)?;
        self.regen.to_binary(binary)?;
        self.unlucky.to_binary(binary)?;
        self.doomed.to_binary(binary)
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
impl std::str::FromStr for Duration {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(' ');
        match split.next() {
            Some("none") => Ok(Duration::None),
            Some("turns") => Ok(Duration::Turns(parse(split.next())?)),
            Some("infinite") => Ok(Duration::Infinite),
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
