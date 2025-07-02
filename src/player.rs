use crate::{Board, Direction, ItemType, Style, Vector, commands::parse, pieces::spell::Stepper};
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
    pub items: [Option<ItemType>; 6],
    pub money: usize,
    pub perception: usize,
    pub effects: Effects,
}
impl Player {
    pub fn new(pos: Vector) -> Player {
        Player {
            pos,
            selector: pos,
            health: 20,
            max_health: 50,
            energy: 3,
            max_energy: 3,
            blocking: false,
            was_hit: false,
            focus: Focus::Player,
            killer: None,
            items: [None; 6],
            money: 0,
            perception: 10,
            effects: Effects::new(),
        }
    }
    pub fn do_move(&mut self, direction: Direction, board: &mut Board) {
        crate::log!("Moving from {} in {direction}", self.pos);
        self.pos += direction;
        if let Some(piece) = &board[self.pos] {
            crate::log!("  Triggering on_step at {}", self.pos);
            if piece.on_step(Stepper::Player(self)) {
                crate::log!("    Removing piece");
                board[self.pos] = None;
            }
        }
    }
    // Returns whether the attack was successful(Ok) and whether the player died
    // true: died
    // false: alive
    pub fn attacked(&mut self, damage: usize, attacker: &'static str) -> Result<bool, ()> {
        if self.effects.invincible.is_active() {
            return Err(());
        }
        self.was_hit = true;
        if self.blocking {
            return Err(());
        }
        if self.health <= damage {
            self.killer = Some(attacker);
            return Ok(true);
        }
        self.health -= damage;
        Ok(false)
    }
    pub fn on_kill(&mut self, variant: crate::enemy::Variant) {
        let (energy, health) = variant.kill_value();
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
            "Killed {variant}, health is now: {}, energy is now: {}",
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
                write!(
                    std::io::stdout(),
                    "\x1b[2J\x1b[15;0HYou were killed by {}{}\x1b[0m.\n",
                    Style::new().green().intense(true).enact(),
                    killer
                )
                .unwrap();
                write!(
                    std::io::stdout(),
                    "{}",
                    match crate::random() & 3 {
                        0 => "Do better next time.",
                        1 => "With enough luck you'll eventually win even without skill.",
                        2 => "You CAN prevail.",
                        3 => "You ever heard of the definition of insanity?",
                        _ => unreachable!("Ain't my problem"),
                    }
                )
                .unwrap();
                write!(
                    std::io::stdout(),
                    "\nPress {}Enter\x1b[0m to exit.",
                    Style::new().cyan().enact()
                )
                .unwrap();
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
            lock.read(&mut buf).unwrap();
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
        if self.health > self.max_health {
            self.health = self.max_health;
        }
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
        write!(lock, "{}{}\x1b[0m", STYLE.enact(), SYMBOL).unwrap();
    }
    fn draw_health(&self, board: &Board, lock: &mut impl std::io::Write) {
        crossterm::queue!(
            lock,
            crossterm::cursor::MoveTo(1, (board.render_y * 2) as u16 + 1)
        )
        .unwrap();
        write!(
            lock,
            "\x1b[2K[\x1b[32m{}\x1b[31m{}\x1b[0m] {}/50",
            "#".repeat(self.health),
            "-".repeat(self.max_health - self.health),
            self.health,
        )
        .unwrap();
    }
    fn draw_energy(&self, board: &Board, lock: &mut impl std::io::Write) {
        crossterm::queue!(
            lock,
            crossterm::cursor::MoveTo(1, (board.render_y * 2) as u16 + 2)
        )
        .unwrap();
        write!(
            lock,
            "\x1b[2K[\x1b[96m{}\x1b[0m{}] {}/3",
            "#".repeat(self.energy * 5),
            "-".repeat((self.max_energy - self.energy) * 5),
            self.energy
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
#[derive(Debug, Clone, Copy)]
pub struct Effects {
    // self explanitory
    pub invincible: Duration,
    // No perception check on enemies, but aggro all mage types
    pub mage_sight: Duration,
    // Heal 2 health per turn
    pub regen: Duration,
}
impl Effects {
    // Creates an instance with no effects
    fn new() -> Effects {
        Effects {
            invincible: Duration::None,
            mage_sight: Duration::None,
            regen: Duration::None,
        }
    }
    // Decreases all effect durations by 1 turn
    fn decriment(&mut self) {
        self.invincible.decriment();
        self.mage_sight.decriment();
        self.regen.decriment();
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
                    other => return Err(format!("{other} is not an effect")),
                }
            }
            None => return Err("No effect specified".to_string()),
        }
        Ok(())
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
