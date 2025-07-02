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
mod items;
use items::ItemType;

use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

static LOG: Mutex<Option<File>> = Mutex::new(None);

// Delay between moves/applicable thinks
const DELAY: std::time::Duration = std::time::Duration::from_millis(100);
// Delay between subtick animaion frames
const PROJ_DELAY: std::time::Duration = std::time::Duration::from_millis(25);

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

// Global flags
use std::sync::atomic::{AtomicBool, Ordering};
// Trigger the enemies to be rechecked for reachability
static RE_FLOOD: AtomicBool = AtomicBool::new(false);
// Load the next level
static LOAD_MAP: AtomicBool = AtomicBool::new(false);
// load the shop
static LOAD_SHOP: AtomicBool = AtomicBool::new(false);

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
            "maptest" => {
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
        board: generate(151, 151, 45, 15, 75).join().unwrap(),
        turn: 0,
        next_map: std::thread::spawn(|| Board::new(10, 10, 10, 10)),
        next_shop: std::thread::spawn(|| Board::new_shop()),
        level: 0,
    };
    generator::DO_DELAY.store(true, Ordering::SeqCst);
    state.next_map = generate(501, 501, 45, 15, 1000);
    let mut command_handler = commands::CommandHandler::new();
    state.board.flood(state.player.pos);
    state.render();
    loop {
        if state.player.handle_death() {
            break;
        }
        if LOAD_MAP.swap(false, Ordering::Relaxed) {
            state.load_next_map();
        }
        // loading the map and loading the shop are mutually exclusive
        else if LOAD_SHOP.swap(false, Ordering::Relaxed) {
            state.load_shop()
        }
        command_handler.handle(&mut state);
        match Input::get() {
            Input::WASD(direction, sprint) => match sprint {
                true => {
                    match direction {
                        Direction::Up => {
                            if state.player.pos.y < 3 {
                                bell(None);
                                continue;
                            }
                        }
                        Direction::Down => {
                            if state.player.pos.y > state.board.y - 2 {
                                bell(None);
                                continue;
                            }
                        }
                        Direction::Left => {
                            if state.player.pos.x < 3 {
                                bell(None);
                                continue;
                            }
                        }
                        Direction::Right => {
                            if state.player.pos.x > state.board.x - 2 {
                                bell(None);
                                continue;
                            }
                        }
                    }
                    if state.player.energy == 0 {
                        bell(None);
                        continue;
                    }
                    let mut checking = state.player.pos + direction;
                    if !state.board.dashable(checking) {
                        bell(None);
                        continue;
                    }
                    checking += direction;
                    if !state.board.dashable(checking) {
                        bell(None);
                        continue;
                    }
                    checking += direction;
                    if state.board.has_collision(checking) {
                        bell(None);
                        continue;
                    }
                    state.attack_enemy(state.player.pos + direction, false, true);
                    state.attack_enemy(checking - direction, false, true);
                    state.player.energy -= 1;
                    state.player.do_move(direction, &mut state.board);
                    state.player.do_move(direction, &mut state.board);
                    state.player.do_move(direction, &mut state.board);
                    state.increment()
                }
                false => {
                    if state.is_valid_move(direction) {
                        state.player.do_move(direction, &mut state.board);
                        state.increment()
                    } else {
                        bell(None)
                    }
                }
            },
            Input::Arrow(direction) => {
                if state.is_on_board(state.player.selector, direction) {
                    state.player.selector += direction;
                    state.board.draw_desc(&state.player, &mut std::io::stdout());
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
                let fail_msg = Style::new().red().enact()
                    + "You can only attack in the 3 by 3 around you\x1b[0m";
                if state.player.pos.x.abs_diff(state.player.selector.x) > 1 {
                    Board::set_desc(&mut std::io::stdout(), &fail_msg);
                    bell(None);
                    std::io::stdout().flush().unwrap();
                    continue;
                }
                if state.player.pos.y.abs_diff(state.player.selector.y) > 1 {
                    Board::set_desc(&mut std::io::stdout(), &fail_msg);
                    bell(None);
                    std::io::stdout().flush().unwrap();
                    continue;
                }
                for (index, enemy) in state.board.enemies.iter_mut().enumerate() {
                    if enemy.try_read().unwrap().pos == state.player.selector {
                        if enemy.try_write().unwrap().attacked(1) {
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
                        state.increment();
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
            Input::Wait => state.increment(),
            Input::SwapFocus => {
                state.player.focus.cycle();
                state.render();
            }
            Input::Enter => {
                if let Some(Piece::Door(door)) = &mut state.board[state.player.selector] {
                    door.open = !door.open;
                    state.increment();
                    RE_FLOOD.store(true, Ordering::Relaxed)
                }
            }
            Input::Item(index) => {
                debug_assert!(index < 7);
                if let Some(item) = state.player.items[index - 1].take() {
                    if item.enact(&mut state) {
                        state.increment()
                    }
                } else {
                    bell(None);
                }
            }
            Input::Convert => {
                state.player.money += state.player.energy;
                state.player.energy = 0;
                state.increment();
            }
        }
        if RE_FLOOD.swap(false, Ordering::Relaxed) {
            state.board.flood(state.player.pos);
            state.render();
        }
    }
}
struct State {
    player: Player,
    board: Board,
    turn: usize,
    next_map: std::thread::JoinHandle<Board>,
    next_shop: std::thread::JoinHandle<Board>,
    level: usize,
}
impl State {
    // returns if an enemy was hit
    fn attack_enemy(&mut self, pos: Vector, redrawable: bool, dashstun: bool) -> bool {
        for (index, enemy) in self.board.enemies.iter_mut().enumerate() {
            if enemy.try_read().unwrap().pos == pos {
                if dashstun {
                    enemy.try_write().unwrap().apply_dashstun()
                }
                if enemy.try_write().unwrap().attacked(1) {
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
        if self.player.effects.regen.is_active() {
            self.player.heal(2)
        }
        self.board.purge_dead();
        self.board.generate_nav_data(self.player.pos);
        let bounds = self.board.get_render_bounds(&self.player);
        for enemy in self.board.enemies.clone().iter() {
            self.board
                .move_and_think(&mut self.player, enemy.clone(), bounds.clone());
        }
        self.board.place_exit();
    }
    fn render(&mut self) {
        let bounds = self.board.get_render_bounds(&self.player);
        self.board.smart_render(&mut self.player);
        self.draw_turn_level_and_money();
        self.player
            .reposition_cursor(self.board.has_background(self.player.selector), bounds);
    }
    fn draw_turn_level_and_money(&self) {
        crossterm::execute!(
            std::io::stdout(),
            crossterm::cursor::MoveTo(1, self.board.render_y as u16 * 2 + 4),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
        )
        .unwrap();
        print!(
            "turn: {}\x1b[30Glayer: {}\x1b[60Gmoney: {}",
            self.turn, self.level, self.player.money
        );
    }
    fn increment(&mut self) {
        self.player.decriment_effects();
        self.think();
        self.turn += 1;
        self.render();
    }
    fn load_next_map(&mut self) {
        generator::DO_DELAY.store(false, Ordering::SeqCst);
        self.board = std::mem::replace(
            &mut self.next_map,
            std::thread::spawn(|| Board::new(1, 1, 1, 1)),
        )
        .join()
        .unwrap();
        generator::DO_DELAY.store(true, Ordering::SeqCst);
        self.next_map = generate(501, 501, 45, 15, self.turn);
        self.level += 1;
        self.player.pos = Vector::new(1, 1);
        self.player.selector = Vector::new(1, 1);
        self.board.flood(self.player.pos);
        self.render();
    }
    fn load_shop(&mut self) {
        self.board = std::mem::replace(
            &mut self.next_shop,
            std::thread::spawn(|| Board::new_shop()),
        )
        .join()
        .unwrap();
        self.player.pos = Vector::new(1, 15);
        self.player.selector = Vector::new(1, 15);
        self.render();
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
        crossterm::execute!(std::io::stdout(), crossterm::terminal::DisableLineWrap).unwrap();
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
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnableLineWrap).unwrap()
    }
}
fn bell(lock: Option<&mut dyn std::io::Write>) {
    let mut buf = [7];
    match lock {
        Some(lock) => {
            lock.write(&mut buf).unwrap();
        }
        None => {
            std::io::stdout().write(&mut buf).unwrap();
            std::io::stdout().flush().unwrap();
        }
    }
}
