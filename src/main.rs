mod player;
use player::Player;
mod board;
use board::{Board, Piece};
mod style;
use style::Style;
mod input;
use input::{Direction, Input};
mod enemy;
mod pieces;
use enemy::Enemy;
mod random;
use random::{Random, random, random_in_range};
mod commands;
mod generator;
use generator::generate;

use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

static LOG: Mutex<Option<File>> = Mutex::new(None);
const DELAY: std::time::Duration = std::time::Duration::from_millis(250);

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        #[cfg(any(debug_assertions, feature = "force_log"))]
        crate::log(format!($($arg)*))
    }
}
#[cfg(any(debug_assertions, feature = "force_log"))]
fn log(string: String) {
    write!(LOG.lock().unwrap().as_ref().unwrap(), "{string}\n").unwrap();
}

use std::sync::atomic::{AtomicBool, Ordering};
static RE_FLOOD: AtomicBool = AtomicBool::new(false);

fn main() {
    #[cfg(any(debug_assertions, feature = "force_log"))]
    {
        *LOG.lock().unwrap() = Some(File::create("log").unwrap());
    }
    random::initialize();
    let mut args = std::env::args();
    let mut testing = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" | "-s" => {
                let new_index = args.next().unwrap().parse().unwrap();
                log!("Setting random index to {new_index}");
                random::initialize_with(new_index)
            }
            "testgen" => {
                log!("TESTING MAP GEN");
                testing = true
            }
            _ => {}
        }
    }
    if testing {
        generate(501, 501, 45, 15, 1000).join().unwrap();
        return;
    }

    let _weirdifier = Weirdifier::new();
    /*crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )
    .unwrap();*/
    let mut state = State {
        player: Player::new(Vector::new(1, 1)),
        board: generate(501, 501, 45, 15, 1000).join().unwrap(),
        turn: 0,
    };
    let mut command_handler = commands::CommandHandler::new();
    state.board.flood(state.player.pos);
    state.render();
    loop {
        if state.player.handle_death() {
            break;
        }
        command_handler.handle(&mut state);
        match Input::get() {
            Input::WASD(direction, sprint) => match sprint {
                true => {
                    match direction {
                        Direction::Up => {
                            if state.player.pos.y < 3 {
                                continue;
                            }
                        }
                        Direction::Down => {
                            if state.player.pos.y > state.board.y - 2 {
                                continue;
                            }
                        }
                        Direction::Left => {
                            if state.player.pos.x < 3 {
                                continue;
                            }
                        }
                        Direction::Right => {
                            if state.player.pos.x > state.board.x - 2 {
                                continue;
                            }
                        }
                    }
                    if state.player.energy == 0 {
                        continue;
                    }
                    let mut checking = state.player.pos + direction;
                    if !state.board.dashable(checking) {
                        continue;
                    }
                    checking += direction;
                    if !state.board.dashable(checking) {
                        continue;
                    }
                    checking += direction;
                    if state.board.has_collision(checking) {
                        continue;
                    }
                    state.attack_enemy(state.player.pos + direction, false, true);
                    state.attack_enemy(checking - direction, false, true);
                    state.player.energy -= 1;
                    state.player.do_move(direction, &mut state.board);
                    state.player.do_move(direction, &mut state.board);
                    state.player.do_move(direction, &mut state.board);
                    state.think();
                    state.turn += 1;
                    state.render()
                }
                false => {
                    if state.is_valid_move(direction) {
                        state.player.do_move(direction, &mut state.board);
                        state.think();
                        state.turn += 1;
                        state.render()
                    }
                }
            },
            Input::Arrow(direction) => {
                if state.is_on_board(state.player.selector, direction) {
                    state.player.selector += direction;
                    state.player.reposition_cursor(
                        state.board.has_background(state.player.selector),
                        state.board.get_render_bounds(&state.player),
                    );
                    if let player::Focus::Selector = state.player.focus {
                        state.render();
                    }
                }
            }
            Input::Attack => {
                if state.player.pos.x.abs_diff(state.player.selector.x) > 1 {
                    continue;
                }
                if state.player.pos.y.abs_diff(state.player.selector.y) > 1 {
                    continue;
                }
                for (index, enemy) in state.board.enemies.iter_mut().enumerate() {
                    if enemy.try_read().unwrap().pos == state.player.selector {
                        if enemy.try_write().unwrap().attacked() {
                            state.player.on_kill(
                                state
                                    .board
                                    .enemies
                                    .swap_remove(index)
                                    .try_read()
                                    .unwrap()
                                    .variant
                                    .clone(),
                            )
                        }
                        state.think();
                        state.turn += 1;
                        state.render();
                        break;
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
            }
            Input::Return => {
                state.player.selector = state.player.pos;
                state
                    .player
                    .reposition_cursor(false, state.board.get_render_bounds(&state.player));
                state.render();
            }
            Input::Wait => {
                state.think();
                state.turn += 1;
                state.render()
            }
            Input::SwapFocus => {
                state.player.focus.cycle();
                state.render();
            }
            Input::Enter => {
                if let Some(Piece::Door(door)) = &mut state.board[state.player.selector] {
                    door.open = !door.open;
                    state.think();
                    state.turn += 1;
                    state.render();
                    RE_FLOOD.store(true, Ordering::Relaxed)
                }
            }
        }
        if RE_FLOOD.swap(false, Ordering::Relaxed) {
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
            if enemy.try_read().unwrap().pos == pos {
                if dashstun {
                    enemy.try_write().unwrap().apply_dashstun()
                }
                if enemy.try_write().unwrap().attacked() {
                    self.player.on_kill(
                        self.board
                            .enemies
                            .swap_remove(index)
                            .try_read()
                            .unwrap()
                            .variant
                            .clone(),
                    );
                    if redrawable {
                        self.render()
                    }
                }
                return true;
            }
        }
        false
    }
    fn is_on_board(&self, start: Vector, direction: Direction) -> bool {
        match direction {
            Direction::Up => {
                if start.y == 0 {
                    return false;
                }
            }
            Direction::Down => {
                if start.y == self.board.y - 1 {
                    return false;
                }
            }
            Direction::Left => {
                if start.x == 0 {
                    return false;
                }
            }
            Direction::Right => {
                if start.x == self.board.x - 1 {
                    return false;
                }
            }
        }
        true
    }
    fn is_valid_move(&self, direction: Direction) -> bool {
        if self.is_on_board(self.player.pos, direction) {
            return !self.board.has_collision(self.player.pos + direction);
        }
        false
    }
    fn think(&mut self) {
        self.board.generate_nav_data(self.player.pos);
        let bounds = self.board.get_render_bounds(&self.player);
        for enemy in self.board.enemies.clone().iter() {
            self.board
                .move_and_think(&mut self.player, enemy.clone(), bounds.clone());
        }
    }
    fn render(&mut self) {
        let bounds = self.board.get_render_bounds(&self.player);
        self.board.render(bounds.clone());
        self.player.draw(&self.board, bounds.clone());
        self.draw_turn();
        self.player
            .reposition_cursor(self.board.has_background(self.player.selector), bounds);
    }
    fn draw_turn(&self) {
        crossterm::execute!(
            std::io::stdout(),
            crossterm::cursor::MoveTo(2, self.board.y as u16 + 3)
        )
        .unwrap();
        print!("turn: {}", self.turn);
    }
}
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct Vector {
    x: usize,
    y: usize,
}
impl Vector {
    fn new(x: usize, y: usize) -> Vector {
        Vector { x, y }
    }
    fn to_move(self) -> crossterm::cursor::MoveTo {
        crossterm::cursor::MoveTo(self.x as u16, self.y as u16)
    }
    fn clamp(self, bounds: std::ops::Range<Vector>) -> Vector {
        let mut out = self;
        if bounds.start.x > out.x {
            out.x = bounds.start.x
        } else if bounds.end.x < out.x {
            out.x = bounds.end.x
        }
        if bounds.start.y > out.y {
            out.y = bounds.start.y
        } else if bounds.end.y < out.y {
            out.y = bounds.end.y
        }
        out
    }
    fn is_near(self, other: Vector, range: usize) -> bool {
        self.x.abs_diff(other.x) < range && self.y.abs_diff(other.y) < range
    }
}
impl std::ops::Sub for Vector {
    type Output = Vector;
    fn sub(self, rhs: Self) -> Self::Output {
        Vector {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl std::ops::Add for Vector {
    type Output = Vector;
    fn add(self, rhs: Self) -> Self::Output {
        Vector {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl std::fmt::Display for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}
impl PartialOrd for Vector {
    fn lt(&self, other: &Self) -> bool {
        self.x.lt(&other.x) && self.y.lt(&other.y)
    }
    fn le(&self, other: &Self) -> bool {
        self.x.le(&other.x) && self.y.le(&other.y)
    }
    fn gt(&self, other: &Self) -> bool {
        self.x.gt(&other.x) && self.y.le(&other.y)
    }
    fn ge(&self, other: &Self) -> bool {
        self.x.ge(&other.x) && self.y.le(&other.y)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self < other {
            Some(std::cmp::Ordering::Less)
        } else if self > other {
            Some(std::cmp::Ordering::Greater)
        } else if self == other {
            Some(std::cmp::Ordering::Equal)
        } else {
            None
        }
    }
}
struct Weirdifier;
impl Weirdifier {
    fn new() -> Weirdifier {
        print!("\x1b[?1049h");
        std::process::Command::new("stty")
            .arg("-icanon")
            .arg("-echo")
            .status()
            .unwrap();
        /*crossterm::execute!(std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen
        ).unwrap();*/
        Weirdifier
    }
}
impl Drop for Weirdifier {
    fn drop(&mut self) {
        print!("\x1b[?1049l");
        std::process::Command::new("stty")
            .arg("icanon")
            .arg("echo")
            .status()
            .unwrap();
        /*crossterm::execute!(std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen
        ).unwrap()*/
    }
}
