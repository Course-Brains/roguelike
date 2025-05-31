mod player;
use player::Player;
mod board;
use board::{Board, Piece};
mod style;
use style::Style;
mod input;
use input::{Input, Direction};
mod pieces;
mod random;
use random::random;

use std::sync::Mutex;
use std::mem::MaybeUninit;
use std::fs::File;
use std::io::Write;
static LOG: Mutex<MaybeUninit<File>> = Mutex::new(MaybeUninit::uninit());
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        #[cfg(any(debug_assertions, feature = "force_log"))]
        crate::log(format!($($arg)*))
    }
}
#[cfg(any(debug_assertions, feature = "force_log"))]
fn log(string: String) {
    unsafe {
        write!(LOG.lock().unwrap().assume_init_mut(), "{string}\n").unwrap()
    }
}

fn main() {
    #[cfg(any(debug_assertions, feature = "force_log"))]
    LOG.lock().unwrap().write(File::create("log").unwrap());
    random::initialize();

    let _weirdifier = Weirdifier::new();
    let mut state = State {
        player: Player::new(Vector::new(3, 3)),
        board: Board::new(90,30)
    };
    state.board.make_room(Vector::new(1,1), Vector::new(30,30));
    state.board[Vector::new(29, 15)] = Some(board::Piece::Door(pieces::door::Door{ open: true }));
    state.board[Vector::new(10, 15)] = Some(Piece::Enemy(pieces::enemy::Enemy::new()));
    state.board.render();
    state.player.draw(&state.board);
    state.player.reposition_cursor();
    loop {
        match Input::get() {
            Input::WASD(direction, sprint) => {
                match sprint {
                    true => {
                        match direction {
                            Direction::Up => {
                                if state.player.pos.y < 3 { continue }
                            }
                            Direction::Down => {
                                if state.player.pos.y > state.board.y-2 { continue }
                            }
                            Direction::Left => {
                                if state.player.pos.x < 3 { continue }
                            }
                            Direction::Right => {
                                if state.player.pos.x > state.board.x-2 { continue }
                            }
                        }
                        if state.player.energy == 0 { continue }
                        let mut checking = state.player.pos+direction;
                        if let Some(piece) = &state.board[checking] {
                            if !piece.dashable() { continue }
                        }
                        checking += direction;
                        if let Some(piece) = &state.board[checking] {
                            if !piece.dashable() { continue }
                        }
                        checking += direction;
                        if let Some(piece) = &state.board[checking] {
                            if piece.has_collision() { continue }
                        }
                        state.attack_enemy(state.player.pos+direction, false, true);
                        state.attack_enemy(checking-direction, false, true);
                        state.player.energy -= 1;
                        state.player.do_move(direction);
                        state.player.do_move(direction);
                        state.player.do_move(direction);
                        state.think();
                        state.board.render();
                        state.player.draw(&state.board);
                        state.player.reposition_cursor();
                    }
                    false => {
                        if state.is_valid_move(direction) {
                            state.player.do_move(direction);
                            state.think();
                            state.board.render();
                            state.player.draw(&state.board);
                            state.player.reposition_cursor();
                        }
                    }
                }
            }
            Input::Arrow(direction) => {
                if state.is_on_board(state.player.selector, direction) {
                    state.player.selector += direction;
                    state.player.reposition_cursor();
                }
            }
            Input::Q => { // attack
                if state.player.selector.x > state.player.pos.x {
                    if state.player.selector.x-1 > state.player.pos.x { continue }
                }
                else {
                    if state.player.pos.x-1 > state.player.selector.x { continue }
                }
                if state.player.selector.y > state.player.pos.y {
                    if state.player.selector.y-1 > state.player.pos.y { continue }
                }
                else {
                    if state.player.pos.y-1 > state.player.selector.y { continue }
                }
                if let Some(Piece::Enemy(_)) = state.board[state.player.selector] {
                    state.attack_enemy(state.player.selector, true, false);
                    state.think();
                    state.render();
                }
            }
            Input::E => { // block
                if state.player.energy != 0 {
                    state.player.was_hit = false;
                    state.player.blocking = true;
                    state.think();
                    if state.player.was_hit {
                        state.player.energy -= 1;
                    }
                    state.player.blocking = false;
                    state.render();
                }
            },
            Input::R => {
                state.player.selector = state.player.pos;
                state.player.reposition_cursor();
            }
            Input::Enter => break,
        }
    }
}
struct State {
    player: Player,
    board: Board
}
impl State {
    // returns if an enemy was hit
    fn attack_enemy(&mut self, pos: Vector, redrawable: bool, dashstun: bool) -> bool {
        match &mut self.board[pos] {
            Some(Piece::Enemy(enemy)) => {
                enemy.apply_dashstun();
                if enemy.attacked() {
                    let variant = enemy.variant;
                    self.board[pos] = None;
                    self.player.on_kill(variant);
                    if redrawable {
                        self.render();
                    }
                }
                true
            }
            _ => false
        }
    }
    fn is_on_board(&self, start: Vector, direction: Direction) -> bool {
        match direction {
            Direction::Up => {
                if start.y == 0 { return false }
            }
            Direction::Down => {
                if start.y == self.board.y-1 { return false }
            }
            Direction::Left => {
                if start.x == 0 { return false }
            }
            Direction::Right => {
                if start.x == self.board.x-1 { return false }
            }
        }
        true
    }
    fn is_valid_move(&self, direction: Direction) -> bool {
        if self.is_on_board(self.player.pos, direction) {
            if let Some(piece) = &self.board[self.player.pos+direction] {
                return !piece.has_collision()
            }
            return true
        }
        false
    }
    fn think(&mut self) {
        let size = Vector::new(self.board.x, self.board.y);
        for x in 0..self.board.x {
            for y in 0..self.board.y {
                if let Some(Piece::Enemy(enemy)) = &mut self.board[Vector::new(x, y)] {
                    enemy.think(Vector::new(x, y), size, &mut self.player)
                }
            }
        }
    }
    fn render(&self) {
        self.board.render();
        self.player.draw(&self.board);
        self.player.reposition_cursor();
    }
}
#[derive(Clone, Copy)]
struct Vector {
    x: usize,
    y: usize
}
impl Vector {
    fn new(x: usize, y: usize) -> Vector {
        Vector {
            x,
            y
        }
    }
}
impl std::ops::Sub for Vector {
    type Output = Vector;
    fn sub(self, rhs: Self) -> Self::Output {
        Vector {
            x: self.x - rhs.x,
            y: self.y - rhs.y
        }
    }
}
impl std::ops::Add for Vector {
    type Output = Vector;
    fn add(self, rhs: Self) -> Self::Output {
        Vector {
            x: self.x + rhs.x,
            y: self.y + rhs.y
        }
    }
}
struct Weirdifier;
impl Weirdifier {
    fn new() -> Weirdifier {
        std::process::Command::new("stty").arg("-icanon").arg("-echo").status().unwrap();
        crossterm::execute!(std::io::stdout(),
            crossterm::cursor::SetCursorStyle::SteadyUnderScore
        ).unwrap();
        Weirdifier
    }
}
impl Drop for Weirdifier {
    fn drop(&mut self) {
        std::process::Command::new("stty").arg("icanon").arg("echo").status().unwrap();
        crossterm::execute!(std::io::stdout(),
            crossterm::cursor::SetCursorStyle::DefaultUserShape
        ).unwrap();
    }
}
