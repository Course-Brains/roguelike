use crate::{Board, Direction, Style, Vector, pieces::spell::Stepper};
use std::io::Write;
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
        }
    }
    pub fn draw(&self, board: &Board, bounds: Range<Vector>) {
        let mut lock = std::io::stdout().lock();
        self.draw_player(&mut lock, bounds);
        self.draw_health(board, &mut lock);
        self.draw_energy(board, &mut lock)
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
    pub fn do_move(&mut self, direction: Direction, board: &mut Board) {
        self.pos += direction;
        if let Some(crate::board::Piece::Spell(_)) = &board[self.pos] {
            if let crate::board::Piece::Spell(spell) = board[self.pos].take().unwrap() {
                spell.on_step(Stepper::Player(self));
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
            if self.health > self.max_health {
                self.health = self.max_health;
                break;
            } else if self.energy != self.max_energy {
                self.energy += 1;
            } else if self.health != self.max_health {
                self.health += health;
            } else {
                break;
            }
        }
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
