use crate::{Vector, Style, Direction, Board};
use std::io::Write;
const SYMBOL: char = '@';
const STYLE: Style = Style::new().cyan().intense(true); 
pub struct Player {
    pub pos: Vector,
    pub selector: Vector,
    pub health: usize,
    pub stamina: usize,
    pub blocking: bool
}
impl Player {
    pub fn new(pos: Vector) -> Player {
        Player {
            pos,
            selector: pos,
            health: 20,
            stamina: 3,
            blocking: false
        }
    }
    pub fn draw(&self, board: &Board) {
        let mut lock = std::io::stdout().lock();
        self.draw_player(&mut lock);
        self.draw_health(board, &mut lock);
        self.draw_stamina(board, &mut lock)
    }
    fn draw_player(&self, lock: &mut impl std::io::Write) {
        crossterm::execute!(lock,
            crossterm::cursor::MoveTo(
                self.pos.x as u16,
                self.pos.y as u16
            )
        ).unwrap();
        write!(lock, "{}{}\x1b[0m", STYLE.enact(), SYMBOL).unwrap();
    }
    fn draw_health(&self, board: &Board, lock: &mut impl std::io::Write) {
        crossterm::execute!(lock,
            crossterm::cursor::MoveTo(1, board.y as u16 + 1)
        ).unwrap();
        write!(lock,
            "[\x1b[32m{}\x1b[31m{}\x1b[0m] {}/50",
            "#".repeat(self.health),
            "-".repeat(50-self.health),
            self.health,
        ).unwrap();
    }
    fn draw_stamina(&self, board: &Board, lock: &mut impl std::io::Write) {
        crossterm::execute!(lock,
            crossterm::cursor::MoveTo(1, board.y as u16 + 2)
        ).unwrap();
        write!(lock,
            "[\x1b[96m{}\x1b[0m{}] {}/3",
            "#".repeat(self.stamina*5),
            "-".repeat((3-self.stamina)*5),
            self.stamina
        ).unwrap();
    }
    pub fn reposition_cursor(&self) {
        crossterm::execute!(std::io::stdout(),
            crossterm::cursor::MoveTo(
                self.selector.x as u16,
                self.selector.y as u16
            )
        ).unwrap();
        std::io::stdout().flush().unwrap();
    }
    pub fn do_move(&mut self, direction: Direction) {
        self.pos += direction;
    }
}
