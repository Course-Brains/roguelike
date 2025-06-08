mod player;
use player::Player;
mod board;
use board::Board;
mod style;
use style::Style;
mod input;
use input::{Input, Direction};
mod pieces;
mod enemy;
use enemy::Enemy;
mod random;
use random::random;
mod commands;
mod events;
use events::EventHandler;

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

use std::sync::atomic::{AtomicBool, Ordering};
static RE_FLOOD: AtomicBool = AtomicBool::new(false);

fn main() {
    #[cfg(any(debug_assertions, feature = "force_log"))]
    LOG.lock().unwrap().write(File::create("log").unwrap());
    random::initialize();

    let _weirdifier = Weirdifier::new();
    let mut state = State {
        player: Player::new(Vector::new(40, 3)),
        board: Board::new(1000, 1000, 45, 15),
        turn: 0,
    };
    let mut command_handler = commands::CommandHandler::new();
    let mut event_handler = EventHandler::new();
    state.board.make_room(Vector::new(1,1), Vector::new(30,30));
    state.board[Vector::new(29, 15)] = Some(board::Piece::Door(pieces::door::Door{ open: true }));
    state.board.enemies.push(Enemy::new(Vector::new(10, 15), enemy::Variant::Basic));
    state.board.flood(state.player.pos);
    state.render();
    loop {
        event_handler.handle(&mut state);
        command_handler.handle(&mut state);
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
                        if !state.board.dashable(checking) { continue }
                        checking += direction;
                        if !state.board.dashable(checking) { continue }
                        checking += direction;
                        if state.board.has_collision(checking) { continue }
                        state.attack_enemy(state.player.pos+direction, false, true);
                        state.attack_enemy(checking-direction, false, true);
                        state.player.energy -= 1;
                        state.player.do_move(direction);
                        state.player.do_move(direction);
                        state.player.do_move(direction);
                        state.think();
                        state.turn += 1;
                        state.render()
                    }
                    false => {
                        if state.is_valid_move(direction) {
                            state.player.do_move(direction);
                            state.think();
                            state.turn += 1;
                            state.render()
                        }
                    }
                }
            }
            Input::Arrow(direction) => {
                if state.is_on_board(state.player.selector, direction) {
                    state.player.selector += direction;
                    let base = state.get_render_base(state.player.pos);
                    state.player.reposition_cursor(
                        state.board.has_background(state.player.selector),
                        base.x..base.x+state.board.render_x*2,
                        base.y..base.y+state.board.render_y*2
                    );
                }
            }
            Input::Attack => {
                for (index, enemy) in state.board.enemies.iter_mut().enumerate() {
                    if enemy.pos == state.player.selector {
                        if enemy.attacked() {
                            state.player.on_kill(state.board.enemies.swap_remove(index).variant)
                        }
                        state.turn += 1;
                        state.render();
                        break
                    }
                }
            }
            Input::Block => {
                if state.player.energy != 0 {
                    state.player.was_hit = false;
                    state.player.blocking = true;
                    state.think();
                    if state.player.was_hit {
                        state.player.energy -= 1;
                    }
                    state.player.blocking = false;
                    state.turn += 1;
                    state.render();
                }
            },
            Input::Return => {
                state.player.selector = state.player.pos;
                let base = state.get_render_base(state.player.pos);
                state.player.reposition_cursor(false,
                    base.x..base.x+state.board.render_x*2,
                    base.y..base.y+state.board.render_y*2
                );
            },
            Input::Wait => {
                state.think();
                state.turn += 1;
                state.render()
            }
            Input::Enter => break,
        }
        if RE_FLOOD.swap(false, Ordering::SeqCst) {
            state.board.flood(state.player.pos);
        }
    }
}
struct State {
    player: Player,
    board: Board,
    turn: usize,
}
impl State {
    // returns if an enemy was hit
    fn attack_enemy(&mut self, pos: Vector, redrawable: bool, dashstun: bool) -> bool {
        for (index, enemy) in self.board.enemies.iter_mut().enumerate() {
            if enemy.pos == pos {
                if dashstun { enemy.apply_dashstun() }
                if enemy.attacked() {
                    self.player.on_kill(
                        self.board.enemies.swap_remove(index).variant
                    );
                    if redrawable { self.render() }
                }
                return true
            }
        }
        false
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
            return !self.board.has_collision(self.player.pos+direction)
        }
        false
    }
    fn think(&mut self) {
        self.board.generate_nav_data(self.player.pos);
        self.board.move_enemies(self.player.pos);
        for enemy in self.board.enemies.iter_mut() {
            enemy.think(
                Vector::new(self.board.x, self.board.y),
                &self.board.backtraces,
                &mut self.player
            )
        }
    }
    fn render(&mut self) {
        let base = self.get_render_base(self.player.pos);
        self.board.render(base);
        self.player.draw(&self.board, base);
        self.draw_turn();
        self.player.reposition_cursor(
            self.board.has_background(self.player.selector),
            base.x..base.x+self.board.render_x*2,
            base.y..base.y+self.board.render_y*2
        );
    }
    fn get_render_base(&self, center: Vector) -> Vector {
        let mut out = center;
        if center.x < self.board.render_x {
            out.x = 0;
        }
        else if self.board.x-center.x < self.board.render_x {
            out.x = self.board.x-(self.board.render_x*2);
        }
        else {
            out.x = center.x-self.board.render_x;
        }
        if center.y < self.board.render_y {
            out.y = 0;
        }
        else if self.board.y-center.y < self.board.render_y {
            out.y = self.board.y-(self.board.render_y*2);
        }
        else {
            out.y = center.y-self.board.render_y;
        }
        out
    }
    fn draw_turn(&self) {
        crossterm::execute!(std::io::stdout(),
            crossterm::cursor::MoveTo(2, self.board.y as u16 + 3)
        ).unwrap();
        print!("turn: {}", self.turn);
    }
}
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
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
impl std::fmt::Display for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}
struct Weirdifier;
impl Weirdifier {
    fn new() -> Weirdifier {
        std::process::Command::new("stty").arg("-icanon").arg("-echo").status().unwrap();
        Weirdifier
    }
}
impl Drop for Weirdifier {
    fn drop(&mut self) {
        std::process::Command::new("stty").arg("icanon").arg("echo").status().unwrap();
    }
}
