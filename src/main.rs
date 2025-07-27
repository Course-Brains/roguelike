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
use random::{Random, random, random_in_range, random4};
mod commands;
mod generator;
use generator::generate;
mod items;
use items::ItemType;
mod upgrades;
use upgrades::Upgrades;
mod spell;
use spell::Spell;

use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex, RwLock};

use albatrice::{FromBinary, ToBinary};

static LOG: Mutex<Option<File>> = Mutex::new(None);

// Delay between moves/applicable thinks
const DELAY: std::time::Duration = std::time::Duration::from_millis(100);
// Delay between subtick animaion frames
const PROJ_DELAY: std::time::Duration = std::time::Duration::from_millis(25);
fn proj_delay() {
    std::thread::sleep(PROJ_DELAY);
}
// The format version of the save data, different versions are incompatible and require a restart
// of the save, but the version will only change on releases, so if the user is not going by
// release, then they could end up with two incompatible save files.
const SAVE_VERSION: u16 = 0;
// the path to the file used for saving and loading
const PATH: &str = "save";

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

// Global trigger flags
use std::sync::atomic::{AtomicBool, Ordering};
// Trigger the enemies to be rechecked for reachability
static RE_FLOOD: AtomicBool = AtomicBool::new(false);
fn re_flood() {
    RE_FLOOD.store(true, Ordering::Relaxed);
}
// Load the next level
static LOAD_MAP: AtomicBool = AtomicBool::new(false);
// load the shop
static LOAD_SHOP: AtomicBool = AtomicBool::new(false);
// Save and quit
static SAVE: AtomicBool = AtomicBool::new(false);

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
        generate(MapGenSettings::new(501, 501, 45, 15, 1000))
            .join()
            .unwrap();
        return;
    }

    let _weirdifier = Weirdifier::new();
    /*crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )
    .unwrap();*/
    let mut state = match std::fs::exists(PATH).unwrap() {
        // There is a save file
        true => State::from_binary(&mut std::fs::File::open(PATH).unwrap()).unwrap(),
        // there is not a save file
        false => State {
            player: Player::new(Vector::new(1, 1)),
            board: generate(MapGenSettings::new(151, 151, 45, 15, 75))
                .join()
                .unwrap(),
            turn: 0,
            next_map: std::thread::spawn(|| Board::new(10, 10, 10, 10)),
            next_map_settings: MapGenSettings::new(501, 501, 45, 15, 1000),
            next_shop: std::thread::spawn(|| Board::new_shop()),
            level: 0,
        },
    };
    // discourage save scumming by making it so that if it closes non-properly then the file is
    // gone
    let _ = std::fs::remove_file(PATH);
    generator::DO_DELAY.store(true, Ordering::SeqCst);
    state.next_map = generate(state.next_map_settings);
    let mut command_handler = commands::CommandHandler::new();
    state.board.flood(state.player.pos);
    state.render();
    loop {
        if state.player.handle_death() {
            break;
        }
        if SAVE.load(Ordering::Relaxed) {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(PATH)
                .unwrap();
            state.to_binary(&mut file).unwrap();
            file.sync_all().unwrap();
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
                let new_pos = {
                    match state.player.fast {
                        true => {
                            let mut pos = state.player.selector;
                            for _ in 0..5 {
                                if state.is_on_board(pos, direction) {
                                    pos += direction;
                                } else {
                                    break;
                                }
                            }
                            pos
                        }
                        false => {
                            if state.is_on_board(state.player.selector, direction) {
                                state.player.selector + direction
                            } else {
                                state.player.selector
                            }
                        }
                    }
                };
                if new_pos != state.player.selector {
                    state.player.selector = new_pos;
                    state.board.draw_desc(&state.player, &mut std::io::stdout());
                    state.player.reposition_cursor(
                        state
                            .board
                            .has_background(state.player.selector, &state.player),
                        state.board.get_render_bounds(&state.player),
                    );
                    if let player::Focus::Selector = state.player.focus {
                        state.render();
                    }
                }
            }
            Input::Attack => {
                let fail_msg = format!(
                    "{}You can only attack in the 3 by 3 around you\x1b[0m",
                    Style::new().red()
                );
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
                                &state.board.enemies.swap_remove(index).try_read().unwrap(),
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
                if let Some(item) = state.player.items[index - 1] {
                    if item.enact(&mut state) {
                        state.player.items[index - 1] = None;
                        state.increment();
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
            Input::Inspect => {
                if state.player.inspect {
                    Board::set_desc(&mut std::io::stdout(), "Inspect mode disabled");
                } else {
                    Board::set_desc(&mut std::io::stdout(), "Inspect mode enabled");
                }
                state.reposition_cursor();
                std::io::stdout().flush().unwrap();
                state.player.inspect ^= true;
            }
            Input::Aim => {
                state.player.aiming ^= true;
                if !state.player.aiming {
                    state.render();
                }
            }
            Input::Fast => {
                state.player.fast ^= true;
                Board::set_desc(
                    &mut std::io::stdout(),
                    match state.player.fast {
                        true => "Fast selector movement enabled",
                        false => "Fast selector movement disabled",
                    },
                );
                state.reposition_cursor();
                std::io::stdout().flush().unwrap();
            }
        }
        if RE_FLOOD.swap(false, Ordering::Relaxed) {
            state.board.flood(state.player.pos);
            state.render();
        }
        if state.player.aiming {
            state.player.aim(&mut state.board);
        }
    }
}
struct State {
    player: Player,
    board: Board,
    turn: usize,
    next_map: std::thread::JoinHandle<Board>,
    next_map_settings: MapGenSettings,
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
                    self.player
                        .on_kill(&self.board.enemies.swap_remove(index).try_read().unwrap());
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
        self.board.update_spells(&mut self.player);
        self.board.place_exit();
    }
    fn render(&mut self) {
        let bounds = self.board.get_render_bounds(&self.player);
        self.board.smart_render(&mut self.player);
        self.draw_turn_level_and_money();
        self.player.reposition_cursor(
            self.board
                .has_background(self.player.selector, &self.player),
            bounds,
        );
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
        let settings = MapGenSettings::new(501, 501, 45, 15, self.turn / 10);
        self.next_map = generate(settings);
        self.next_map_settings = settings;
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
        self.player.pos = Vector::new(44, 14);
        self.player.selector = Vector::new(44, 14);
        self.render();
    }
    fn reposition_cursor(&mut self) {
        self.player.reposition_cursor(
            self.board
                .has_background(self.player.selector, &self.player),
            self.board.get_render_bounds(&self.player),
        );
    }
}
impl FromBinary for State {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        if u16::from_binary(binary)? != SAVE_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid save format"),
            ));
        }
        generator::DO_DELAY.store(true, Ordering::SeqCst);
        let settings = MapGenSettings::from_binary(binary)?;
        Ok(State {
            player: Player::from_binary(binary)?,
            board: Board::from_binary(binary)?,
            turn: usize::from_binary(binary)?,
            next_map: generate(settings),
            next_map_settings: settings,
            next_shop: std::thread::spawn(|| Board::new_shop()),
            level: usize::from_binary(binary)?,
        })
    }
}
impl ToBinary for State {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        SAVE_VERSION.to_binary(binary)?;
        self.next_map_settings.to_binary(binary)?;
        self.player.to_binary(binary)?;
        self.board.to_binary(binary)?;
        self.turn.to_binary(binary)?;
        self.level.to_binary(binary)
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
    fn list_near(self, range: usize) -> Vec<Vector> {
        let range = range as isize;
        let mut out = Vec::new();
        for x in -range..=range {
            if x < 0 {
                if x.abs_diff(0) > self.x {
                    continue;
                }
            }
            for y in -range..=range {
                if y < 0 {
                    if y.abs_diff(0) > self.y {
                        continue;
                    }
                }
                if x == 0 && y == 0 {
                    continue;
                }
                out.push(Vector::new(
                    (self.x as isize + x) as usize,
                    (self.y as isize + y) as usize,
                ));
            }
        }
        out
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
impl FromBinary for Vector {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Vector::new(
            usize::from_binary(binary)?,
            usize::from_binary(binary)?,
        ))
    }
}
impl ToBinary for Vector {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.x.to_binary(binary)?;
        self.y.to_binary(binary)?;
        Ok(())
    }
}
enum Entity<'a> {
    Player(&'a mut Player),
    Enemy(Arc<RwLock<Enemy>>),
}
impl<'a> Entity<'a> {
    fn new(src: Option<Arc<RwLock<Enemy>>>, player: &'a mut Player) -> Self {
        match src {
            Some(enemy) => enemy.into(),
            None => player.into(),
        }
    }
    /*fn unwrap_player(self) -> &'a mut Player {
        match self {
            Self::Player(player) => player,
            Self::Enemy(_) => panic!("Expected player, found enemy"),
        }
    }*/
    fn unwrap_enemy(self) -> Arc<RwLock<Enemy>> {
        match self {
            Self::Player(_) => panic!("Expected enemy, found player"),
            Self::Enemy(enemy) => enemy,
        }
    }
    /*fn get_pos(&self) -> Vector {
        match self {
            Entity::Enemy(arc) => arc.try_read().unwrap().pos,
            Entity::Player(player) => player.pos,
        }
    }
    fn is_within_flood(&self) -> bool {
        match self {
            Entity::Player(_) => true,
            Entity::Enemy(arc) => arc.try_read().unwrap().reachable,
        }
    }
    fn is_player(&self) -> bool {
        match self {
            Entity::Player(_) => true,
            Entity::Enemy(_) => false,
        }
    }
    fn is_entity(&self) -> bool {
        match self {
            Entity::Player(_) => false,
            Entity::Enemy(_) => true,
        }
    }*/
}
impl<'a> From<&'a mut Player> for Entity<'a> {
    fn from(value: &'a mut Player) -> Self {
        Entity::Player(value)
    }
}
impl From<Arc<RwLock<Enemy>>> for Entity<'_> {
    fn from(value: Arc<RwLock<Enemy>>) -> Self {
        Entity::Enemy(value)
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
#[derive(Clone, Copy, Debug)]
struct MapGenSettings {
    x: usize,
    y: usize,
    render_x: usize,
    render_y: usize,
    budget: usize,
}
impl MapGenSettings {
    fn new(x: usize, y: usize, render_x: usize, render_y: usize, budget: usize) -> MapGenSettings {
        Self {
            x,
            y,
            render_x,
            render_y,
            budget,
        }
    }
}
impl FromBinary for MapGenSettings {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            x: usize::from_binary(binary)?,
            y: usize::from_binary(binary)?,
            render_x: usize::from_binary(binary)?,
            render_y: usize::from_binary(binary)?,
            budget: usize::from_binary(binary)?,
        })
    }
}
impl ToBinary for MapGenSettings {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.x.to_binary(binary)?;
        self.y.to_binary(binary)?;
        self.render_x.to_binary(binary)?;
        self.render_y.to_binary(binary)?;
        self.budget.to_binary(binary)
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
fn advantage_pass(pass: impl Fn() -> bool, modifier: isize) -> bool {
    match modifier.cmp(&0) {
        std::cmp::Ordering::Less => {
            // negative(disadvantage)
            for _ in 0..(modifier.abs() + 1) {
                if !pass() {
                    return false;
                }
            }
            true
        }
        std::cmp::Ordering::Greater => {
            // positive(advantage)
            for _ in 0..(modifier + 1) {
                if pass() {
                    return true;
                }
            }
            false
        }
        std::cmp::Ordering::Equal => pass(),
    }
}
fn set_desc(msg: &'static str) {
    Board::set_desc(&mut std::io::stdout(), msg);
    std::io::stdout().flush().unwrap();
}
// Gets the list of positions a projectile travels through and what it hit
fn ray_cast(
    from: Vector,
    to: Vector,
    board: &Board,
    addr: Option<usize>,
    end_stop: bool,
    player: Vector,
) -> (Vec<Vector>, Option<Collision>) {
    crate::log!("calculating projectile path from {from} to {to}");
    let x = to.x as f64 - from.x as f64;
    let y = to.y as f64 - from.y as f64;
    let len = (x.powi(2) + y.powi(2)).sqrt();
    let delta_x = (x / len) / 2.0;
    let delta_y = (y / len) / 2.0;
    crate::log!("  Will move ({delta_x}, {delta_y}) per calc");
    let mut precise_x = from.x as f64;
    let mut precise_y = from.y as f64;
    let mut x = from.x;
    let mut y = from.y;
    let mut out = Vec::new();
    let mut collision = None;
    loop {
        if end_stop {
            if x == to.x && y == to.y {
                break;
            }
            if delta_x.is_sign_positive() {
                if x > to.x {
                    break;
                }
            } else {
                if x < to.x {
                    break;
                }
            }
            if delta_y.is_sign_positive() {
                if y > to.y {
                    break;
                }
            } else {
                if y < to.y {
                    break;
                }
            }
        }
        crate::log!("  at ({precise_x}, {precise_y})");
        precise_x += delta_x;
        precise_y += delta_y;
        x = precise_x as usize;
        y = precise_y as usize;
        let pos = Vector::new(x, y);

        if !out.last().is_some_and(|prev| *prev == Vector::new(x, y)) {
            crate::log!("  new position, adding to output");
            out.push(Vector::new(x, y));
        }

        if pos == from {
            continue;
        }
        if let Some(piece) = &board[pos] {
            if piece.projectile_collision() {
                crate::log!("    hit {piece}, stopping");
                collision = Some(pos.into());
                break;
            }
        }
        if let Some(enemy) = board.get_enemy(pos, addr) {
            crate::log!("    hit enemy, stopping");
            collision = Some(enemy.into());
            break;
        }
        if pos == player {
            crate::log!("    hit player, stopping");
            collision = Some(Collision::Player);
            break;
        }
    }
    (out, collision)
}
enum Collision {
    Player,
    Enemy(Arc<RwLock<Enemy>>),
    Piece(Vector),
}
impl Collision {
    fn to_entity<'a>(self, player: &'a mut Player) -> Option<Entity<'a>> {
        match self {
            Self::Player => Some(Entity::Player(player)),
            Self::Enemy(arc) => Some(Entity::Enemy(arc)),
            Self::Piece(_) => None,
        }
    }
}
impl From<Arc<RwLock<Enemy>>> for Collision {
    fn from(value: Arc<RwLock<Enemy>>) -> Self {
        Collision::Enemy(value)
    }
}
impl From<Vector> for Collision {
    fn from(value: Vector) -> Self {
        Collision::Piece(value)
    }
}
