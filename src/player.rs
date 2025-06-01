use crate::{Vector, Style, Direction, Board};
use std::io::Write;
const SYMBOL: char = '@';
const STYLE: Style = *Style::new().cyan().intense(true); 
pub struct Player {
    pub pos: Vector,
    pub selector: Vector,
    pub health: usize,
    max_health: usize,
    pub energy: usize,
    max_energy: usize,
    pub blocking: bool,
    pub was_hit: bool
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
            was_hit: false
        }
    }
    pub fn draw(&self, board: &Board) {
        let mut lock = std::io::stdout().lock();
        self.draw_player(&mut lock);
        self.draw_health(board, &mut lock);
        self.draw_energy(board, &mut lock)
    }
    fn draw_player(&self, lock: &mut impl std::io::Write) {
        crossterm::queue!(lock,
            crossterm::cursor::MoveTo(
                self.pos.x as u16,
                self.pos.y as u16
            )
        ).unwrap();
        write!(lock, "{}{}\x1b[0m", STYLE.enact(), SYMBOL).unwrap();
    }
    fn draw_health(&self, board: &Board, lock: &mut impl std::io::Write) {
        crossterm::queue!(lock,
            crossterm::cursor::MoveTo(1, board.y as u16 + 1)
        ).unwrap();
        write!(lock,
            "[\x1b[32m{}\x1b[31m{}\x1b[0m] {}/50",
            "#".repeat(self.health),
            "-".repeat(self.max_health-self.health),
            self.health,
        ).unwrap();
    }
    fn draw_energy(&self, board: &Board, lock: &mut impl std::io::Write) {
        crossterm::queue!(lock,
            crossterm::cursor::MoveTo(1, board.y as u16 + 2)
        ).unwrap();
        write!(lock,
            "[\x1b[96m{}\x1b[0m{}] {}/3",
            "#".repeat(self.energy*5),
            "-".repeat((self.max_energy-self.energy)*5),
            self.energy
        ).unwrap();
    }
    pub fn reposition_cursor(&self, underscore: bool) {
        crossterm::execute!(std::io::stdout(),
            crossterm::cursor::MoveTo(
                self.selector.x as u16,
                self.selector.y as u16
            )
        ).unwrap();
        if underscore {
            crossterm::execute!(std::io::stdout(),
                crossterm::cursor::SetCursorStyle::SteadyUnderScore
            ).unwrap()
        }
        else {
            crossterm::execute!(std::io::stdout(),
                crossterm::cursor::SetCursorStyle::DefaultUserShape
            ).unwrap()
        }
        std::io::stdout().flush().unwrap();
    }
    pub fn do_move(&mut self, direction: Direction) {
        self.pos += direction;
    }
    // Returns whether the attack was successful(Ok) and whether the player died
    // true: died
    // false: alive
    pub fn attacked(&mut self, damage: usize) -> Result<bool,()> {
        self.was_hit = true;
        if self.blocking { return Err(()) }
        if self.health <= damage { return Ok(true) }
        self.health -= damage;
        Ok(false)
    }
    pub fn on_kill(&mut self, variant: crate::pieces::enemy::Variant) {
        let (energy, health) = variant.kill_value();
        for _ in 0..energy {
            if self.energy != self.max_energy {
                self.energy += 1;
            }
            else if self.health != self.max_health {
                self.health += health;
            }
            else {
                break
            }
        }
    }
}
