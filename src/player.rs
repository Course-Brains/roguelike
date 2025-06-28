use crate::{Board, Direction, ItemType, Style, Vector, pieces::spell::Stepper};
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
            items: [Some(ItemType::Testing); 6],
            money: 0,
        }
    }
    pub fn do_move(&mut self, direction: Direction, board: &mut Board) {
        self.pos += direction;
        if let Some(piece) = &board[self.pos] {
            if piece.on_step(Stepper::Player(self)) {
                board[self.pos] = None;
            }
        }
    }
    // Returns whether the attack was successful(Ok) and whether the player died
    // true: died
    // false: alive
    pub fn attacked(&mut self, damage: usize, attacker: &'static str) -> Result<bool, ()> {
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
                    "\x1b[2J\x1b[15;0HYou were killed by {}{}\x1b[0m. Do better next time.\nPress enter to exit.",
                    Style::new().green().intense(true).enact(),
                    killer
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
        let mut buf = [0];
        let mut lock = std::io::stdin().lock();
        let selected = loop {
            lock.read(&mut buf).unwrap();
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
                self.items[index] = Some(item);
                true
            }
            None => false,
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
