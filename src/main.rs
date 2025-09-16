// When I do add spells, add a system for random unidentifiable buffs that get determined at the
// start, with one of them being the ability to do other actions while casting
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
use random::{Rand, Random, random};
mod commands;
mod generator;
use generator::generate;
mod items;
use items::ItemType;
mod upgrades;
use upgrades::Upgrades;
mod spell;
use spell::Spell;
mod limbs;

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, LazyLock, Mutex, RwLock};

use albatrice::{FromBinary, ToBinary};

// Convenience constant
const RELAXED: Ordering = Ordering::Relaxed;

// The format version of the save data, different versions are incompatible and require a restart
// of the save, but the version will only change on releases, so if the user is not going by
// release, then they could end up with two incompatible save files.
const SAVE_VERSION: Version = 6;
// The number that the turn count is divided by to get the budget
const BUDGET_DIVISOR: usize = 5;
// The number of bosses in each level starting at the third level
const NUM_BOSSES: usize = 5;

static LOG: Mutex<Option<File>> = Mutex::new(None);
static STATS: LazyLock<Mutex<Stats>> = LazyLock::new(|| Mutex::new(Stats::new()));
fn stats<'a>() -> std::sync::MutexGuard<'a, Stats> {
    STATS.lock().unwrap()
}
// Whether or not the console was used
static CHEATS: AtomicBool = AtomicBool::new(false);
mod bench {
    use std::fs::File;
    use std::sync::{LazyLock, RwLock, RwLockWriteGuard, atomic::AtomicBool};
    // Whether or not to esspecially log timings
    pub static BENCHMARK: AtomicBool = AtomicBool::new(false);
    macro_rules! bench_maker {
        ($path: literal) => {
            LazyLock::new(|| RwLock::new(File::create($path).unwrap()))
        };
        ($name: ident, $index: literal) => {
            pub fn $name<'a>() -> RwLockWriteGuard<'a, File> {
                BENCHES.get($index).unwrap().write().unwrap()
            }
        };
    }
    static BENCHES: [LazyLock<RwLock<File>>; 7] = [
        bench_maker!("bench/render"),
        bench_maker!("bench/vis_flood"),
        bench_maker!("bench/flood"),
        bench_maker!("bench/nav"),
        bench_maker!("bench/think"),
        bench_maker!("bench/open_flood"),
        bench_maker!("bench/total"),
    ];
    bench_maker!(render, 0);
    bench_maker!(vis_flood, 1);
    bench_maker!(flood, 2);
    bench_maker!(nav, 3);
    bench_maker!(think, 4);
    bench_maker!(open_flood, 5);
    bench_maker!(total, 6);
}
fn enable_benchmark() {
    bench::BENCHMARK.store(true, Ordering::SeqCst);
    if !std::fs::exists("bench").unwrap() {
        std::fs::create_dir("bench").unwrap();
    }
}
fn bench() -> bool {
    bench::BENCHMARK.load(Ordering::SeqCst)
}

// Delay between moves/applicable thinks
const DELAY: std::time::Duration = std::time::Duration::from_millis(100);
// Delay between subtick animaion frames
const PROJ_DELAY: std::time::Duration = std::time::Duration::from_millis(25);
fn proj_delay() {
    std::thread::sleep(PROJ_DELAY);
}
type Version = u32;
// the path to the file used for saving and loading
const PATH: &str = "save";
// The path to the file of stats for previous runs
const STAT_PATH: &str = "stats";

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        $crate::log(format!($($arg)*))
    }
}
#[cfg(feature = "log")]
fn log(string: String) {
    writeln!(LOG.lock().unwrap().as_ref().unwrap(), "{string}").unwrap();
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! debug_only {
    ($val:tt) => {
        compile_error!("Someone forgot to replace the placeholder value!")
    };
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! debug_only {
    ($val:tt) => {
        $val
    };
}

//////////////////////////
// Global trigger flags //
//////////////////////////
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

///////////////////////
// Global data flags //
///////////////////////
static BONUS_NO_DAMAGE: AtomicBool = AtomicBool::new(true);
static BONUS_NO_WASTE: AtomicBool = AtomicBool::new(true);
static BONUS_NO_ENERGY: AtomicBool = AtomicBool::new(true);
fn reset_bonuses() {
    BONUS_NO_DAMAGE.store(true, Ordering::Relaxed);
    BONUS_NO_WASTE.store(true, Ordering::Relaxed);
    BONUS_NO_ENERGY.store(true, RELAXED);
}

/////////////////////////////
// General purpose globals //
/////////////////////////////

// Stores specials that will last one turn and will be reset at the end of the current turn, every
// turn.
static ONE_TURN_SPECIALS: Mutex<Vec<Arc<board::Special>>> = Mutex::new(Vec::new());
// Consistently rendered feedback for the player so that they know more confusing details
static FEEDBACK: Mutex<String> = Mutex::new(String::new());
fn feedback<'a>() -> std::sync::MutexGuard<'a, String> {
    FEEDBACK.lock().unwrap()
}
fn set_feedback(new: String) {
    *FEEDBACK.lock().unwrap() = new;
}
fn draw_feedback() {
    crossterm::queue!(
        std::io::stdout(),
        crossterm::cursor::MoveTo(0, 35),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
    )
    .unwrap();
    write!(std::io::stdout(), "{}", feedback()).unwrap();
    std::io::stdout().flush().unwrap();
}

///////////////////
// Debug toggles //
///////////////////
static SHOW_REACHABLE: AtomicBool = AtomicBool::new(false);
fn show_reachable() -> bool {
    SHOW_REACHABLE.load(RELAXED)
}

/////////////////
// Actual code //
/////////////////
fn main() {
    #[cfg(feature = "log")]
    {
        *LOG.lock().unwrap() = Some(File::create("log").unwrap());
    }
    random::initialize();
    crate::log!(
        "Recieved args: {:?}",
        std::env::args().collect::<Vec<String>>()
    );
    let mut args = std::env::args();
    let mut counting = false;
    let mut testing = false;
    let mut empty = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" | "-s" => {
                let new_index = args.next().unwrap().parse().unwrap();
                log!("Setting random index to {new_index}");
                random::initialize_with(new_index)
            }
            "maptest" => {
                log!("TESTING MAP GEN");
                testing = true;
            }
            "mapcount" => {
                log!("COUNTING BOSSES");
                counting = true
            }
            "stats" => {
                view_stats();
                return;
            }
            "--bench" => {
                log!("Enabling benchmark through command line argument");
                enable_benchmark();
            }
            "empty" => {
                log!("Loading debug map and disabling stats");
                CHEATS.store(true, Ordering::Relaxed);
                empty = true;
            }
            "--no-stats" => CHEATS.store(true, Ordering::Relaxed),
            _ => {}
        }
    }
    if testing {
        generate(MapGenSettings::new(151, 151, 45, 15, 75, 1))
            .join()
            .unwrap();
        return;
    }
    if counting {
        let mut basic = 0;
        let mut mage = 0;
        let mut fighter = 0;
        let mut archer = 0;
        for index in 0..Rand::MAX {
            random::initialize_with(index);
            let board = generate(MapGenSettings::new(151, 151, 45, 15, 75, 1))
                .join()
                .unwrap();
            match board.bosses[0]
                .sibling
                .upgrade()
                .unwrap()
                .try_read()
                .unwrap()
                .variant
            {
                enemy::Variant::BasicBoss(_) => basic += 1,
                enemy::Variant::MageBoss(_) => mage += 1,
                enemy::Variant::FighterBoss { .. } => fighter += 1,
                enemy::Variant::ArcherBoss(_) => archer += 1,
                _ => unreachable!("non boss boss"),
            }
        }
        println!("basic: {basic}\nmage: {mage}\nfighter: {fighter}\narcher: {archer}");
        return;
    }

    let _weirdifier = Weirdifier::new();
    /*crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )
    .unwrap();*/
    let save_file = std::fs::exists(PATH).unwrap();
    let mut state = match save_file {
        // There is a save file
        true => State::from_binary(&mut std::fs::File::open(PATH).unwrap()).unwrap(),
        // there is not a save file
        false => State::new(empty),
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
        if let Some(show_stats) = Player::handle_death(&state) {
            stats().collect_death(&state);
            save_stats();
            if show_stats {
                log!("Showing end of game stats:\n{:#?}", stats());
                println!("\n\n\n{:#?}\n\n\nPress enter to quit.", stats());
                std::io::stdin().read_line(&mut String::new()).unwrap();
            }
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
            stats().num_saves += 1;
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
            Input::Wasd(direction, sprint) => match sprint {
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
                    BONUS_NO_ENERGY.store(false, RELAXED);
                    state.attack_enemy(state.player.pos + direction, false, true, false);
                    state.attack_enemy(checking - direction, false, true, false);
                    state.player.energy -= 1;
                    state.player.do_move(direction, &mut state.board);
                    state.player.do_move(direction, &mut state.board);
                    state.player.do_move(direction, &mut state.board);
                    stats().energy_used += 1;
                    state.increment()
                }
                false => {
                    if state.player.fast {
                        for _ in 0..5 {
                            if state.is_valid_move(direction) {
                                state.player.do_move(direction, &mut state.board);
                                state.increment();
                            }
                        }
                    } else {
                        if state.is_valid_move(direction) {
                            state.player.do_move(direction, &mut state.board);
                            state.increment()
                        } else if state
                            .board
                            .get_enemy(state.player.pos + direction, None)
                            .is_some()
                        {
                            state.attack_enemy(state.player.pos + direction, false, false, true);
                            state.increment();
                        } else if let Some(board::Piece::Door(door)) =
                            state.board[state.player.pos + direction]
                        {
                            if !door.open {
                                state.open_door(state.player.pos + direction, true);
                            }
                        }
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
                if state.board.contains_enemy(state.player.selector, None)
                    && state.attack_enemy(state.player.selector, false, false, false)
                {
                    state.increment();
                }
            }
            Input::Block => {
                if state.player.energy != 0 {
                    BONUS_NO_ENERGY.store(false, RELAXED);
                    stats().energy_used += 1;
                    state.player.was_hit = false;
                    state.player.blocking = true;
                    state.increment();
                    if state.player.was_hit {
                        state.player.energy -= 1;
                    }
                    state.player.blocking = false;
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
                state.open_door(state.player.selector, false);
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
                if state.player.upgrades.precise_convert {
                    if state.player.energy > 0 {
                        state.player.give_money(1);
                        state.player.energy -= 1;
                    }
                } else {
                    state.player.give_money(state.player.energy);
                    state.player.energy = 0;
                }
                state.increment();
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
                        true => "Fast movement enabled",
                        false => "Fast movement disabled",
                    },
                );
                state.reposition_cursor();
                std::io::stdout().flush().unwrap();
            }
            Input::ClearFeedback => {
                *feedback() = String::new();
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
    // debugging
    nav_stepthrough: bool,
    nav_stepthrough_index: Option<usize>,
}
impl State {
    fn new(empty: bool) -> State {
        State {
            player: Player::new(Vector::new(1, 1)),
            board: match empty {
                true => std::thread::spawn(Board::new_empty),
                false => generate(MapGenSettings::new(151, 151, 45, 15, 75, 1)),
            }
            .join()
            .unwrap(),
            turn: 0,
            next_map: std::thread::spawn(|| Board::new(10, 10, 10, 10)),
            next_map_settings: MapGenSettings::new(501, 501, 45, 15, 2000, 3),
            next_shop: std::thread::spawn(Board::new_shop),
            level: 0,
            nav_stepthrough: false,
            nav_stepthrough_index: None,
        }
    }
    // returns if an enemy was hit
    fn attack_enemy(
        &mut self,
        pos: Vector,
        redrawable: bool,
        dashstun: bool,
        walking: bool,
    ) -> bool {
        for (index, enemy) in self.board.enemies.iter_mut().enumerate() {
            if enemy.try_read().unwrap().pos == pos {
                if dashstun {
                    enemy.try_write().unwrap().apply_dashstun()
                }
                if enemy
                    .try_write()
                    .unwrap()
                    .attacked(self.player.get_damage())
                {
                    self.player
                        .on_kill(&self.board.enemies.swap_remove(index).try_read().unwrap());
                    if redrawable {
                        self.render()
                    }
                }
                stats().damage_dealt += self.player.get_damage();
                stats().attacks_done += 1;
                if walking {
                    stats().enemies_hit_by_walking += 1;
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
    fn think(&mut self, time: &mut std::time::Duration) {
        if self.player.effects.regen.is_active() {
            self.player.heal(2)
        }
        self.board.generate_nav_data(
            self.player.pos,
            self.nav_stepthrough,
            self.nav_stepthrough_index,
            &mut self.player,
        );
        let bounds = self.board.get_render_bounds(&self.player);
        let visible = self
            .board
            .get_visible_indexes(bounds.clone(), self.player.effects.full_vis.is_active());
        for (index, enemy) in self.board.enemies.clone().iter().enumerate() {
            self.board.move_and_think(
                &mut self.player,
                enemy.clone(),
                bounds.clone(),
                time,
                visible
                    .last()
                    .is_some_and(|last_index| *last_index == index),
            );
        }
        self.board.purge_dead();
        if bench() {
            writeln!(bench::think(), "{}", time.as_millis()).unwrap();
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
            self.turn,
            self.level,
            self.player.get_money()
        );
    }
    fn increment(&mut self) {
        let mut start = std::time::Instant::now();
        self.player.decriment_effects();
        let mut time = start.elapsed();
        self.think(&mut time);
        start = std::time::Instant::now();
        self.turn += 1;
        self.board.turns_spent += 1;
        self.render();
        *ONE_TURN_SPECIALS.lock().unwrap() = Vec::new();
        self.board.reset_took_damage();
        time += start.elapsed();
        if bench() {
            writeln!(bench::total(), "{}", time.as_millis()).unwrap();
        }
    }
    fn load_next_map(&mut self) {
        generator::DO_DELAY.store(false, Ordering::SeqCst);
        stats().shop_turns.push(self.board.turns_spent);
        self.board = std::mem::replace(
            &mut self.next_map,
            std::thread::spawn(|| Board::new(1, 1, 1, 1)),
        )
        .join()
        .unwrap();
        generator::DO_DELAY.store(true, Ordering::SeqCst);
        let settings =
            MapGenSettings::new(501, 501, 45, 15, self.turn / BUDGET_DIVISOR, NUM_BOSSES);
        reset_bonuses();
        self.next_map = generate(settings);
        self.next_map_settings = settings;
        self.level += 1;
        self.player.pos = Vector::new(1, 1);
        self.player.selector = Vector::new(1, 1);
        self.board.flood(self.player.pos);
        self.render();
    }
    fn load_shop(&mut self) {
        stats().level_turns.push(self.board.turns_spent);
        let bonus_kill_all = self.board.enemies.len() == 0;
        self.board = std::mem::replace(&mut self.next_shop, std::thread::spawn(Board::new_shop))
            .join()
            .unwrap();
        self.player.pos = Vector::new(44, 14);
        self.player.selector = Vector::new(44, 14);

        // bonuses
        if BONUS_NO_WASTE.load(RELAXED) {
            self.board[Board::BONUS_NO_WASTE] = Some(board::Piece::Upgrade(
                pieces::upgrade::Upgrade::new(Some(upgrades::UpgradeType::BonusNoWaste)),
            ));
        }
        if BONUS_NO_DAMAGE.load(Ordering::Relaxed) {
            self.board[Board::BONUS_NO_DAMAGE] = Some(board::Piece::Upgrade(
                pieces::upgrade::Upgrade::new(Some(upgrades::UpgradeType::BonusNoDamage)),
            ));
        }
        if bonus_kill_all {
            self.board[Board::BONUS_KILL_ALL] = Some(board::Piece::Upgrade(
                pieces::upgrade::Upgrade::new(Some(upgrades::UpgradeType::BonusKillAll)),
            ));
        }
        if BONUS_NO_ENERGY.load(RELAXED) {
            self.board[Board::BONUS_NO_ENERGY] = Some(board::Piece::Upgrade(
                pieces::upgrade::Upgrade::new(Some(upgrades::UpgradeType::BonusNoEnergy)),
            ));
        }

        stats().shop_money.push(self.player.get_money());
        self.render();
    }
    fn reposition_cursor(&mut self) {
        self.player.reposition_cursor(
            self.board
                .has_background(self.player.selector, &self.player),
            self.board.get_render_bounds(&self.player),
        );
    }
    fn is_visible(&self, pos: Vector) -> bool {
        self.board.is_visible(
            pos,
            self.board.get_render_bounds(&self.player),
            self.player.effects.full_vis.is_active(),
        )
    }
    fn open_door(&mut self, pos: Vector, walking: bool) {
        if let Some(Piece::Door(door)) = &mut self.board[pos] {
            // Closing the door
            if door.open {
                door.open = false;
                stats().doors_closed += 1;

                let reachable_bosses: Vec<Vector> = self
                    .board
                    .bosses
                    .iter()
                    .filter(|boss| boss.sibling.upgrade().is_some())
                    .map(|boss| boss.last_pos)
                    .collect::<Vec<Vector>>()
                    .iter()
                    .filter(|pos| self.board.is_reachable(**pos))
                    .map(|pos| *pos)
                    .collect();
                if reachable_bosses.len() != 0 {
                    self.board.flood(self.player.pos);
                    if reachable_bosses
                        .iter()
                        .any(|pos| self.board.is_reachable(*pos))
                    {
                        stats().cowardice += 1;
                    }
                    return;
                }
                // we don't need to explicitly set the closed door as unreachable because
                // the flood will do that for us
                re_flood();
            // Opening the door
            } else {
                stats().doors_opened += 1;
                if walking {
                    stats().doors_opened_by_walking += 1;
                }
                self.board.open_door_flood(pos);
                self.board[pos] = Some(Piece::Door(pieces::door::Door { open: true }));
            }
            self.increment();
        }
    }
}
impl FromBinary for State {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        if Version::from_binary(binary)? != SAVE_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid save format".to_string(),
            ));
        }
        CHEATS.store(bool::from_binary(binary)?, Ordering::Relaxed);
        generator::DO_DELAY.store(true, Ordering::SeqCst);
        let settings = MapGenSettings::from_binary(binary)?;
        Ok(State {
            player: Player::from_binary(binary)?,
            board: Board::from_binary(binary)?,
            turn: usize::from_binary(binary)?,
            next_map: generate(settings),
            next_map_settings: settings,
            next_shop: std::thread::spawn(Board::new_shop),
            level: usize::from_binary(binary)?,
            nav_stepthrough: bool::from_binary(binary)?,
            nav_stepthrough_index: Option::from_binary(binary)?,
        })
    }
}
impl ToBinary for State {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        SAVE_VERSION.to_binary(binary)?;
        CHEATS.load(Ordering::Relaxed).to_binary(binary)?;
        self.next_map_settings.to_binary(binary)?;
        self.player.to_binary(binary)?;
        self.board.to_binary(binary)?;
        self.turn.to_binary(binary)?;
        self.level.to_binary(binary)?;
        self.nav_stepthrough.to_binary(binary)?;
        self.nav_stepthrough_index.as_ref().to_binary(binary)
    }
}
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct Vector {
    x: usize,
    y: usize,
}
impl Vector {
    const fn new(x: usize, y: usize) -> Vector {
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
    fn up(self) -> Vector {
        Vector::new(self.x, self.y - 1)
    }
    fn down(self) -> Vector {
        Vector::new(self.x, self.y + 1)
    }
    fn down_mut(&mut self) -> &mut Self {
        self.y += 1;
        self
    }
    fn left(self) -> Vector {
        Vector::new(self.x - 1, self.y)
    }
    fn right(self) -> Vector {
        Vector::new(self.x + 1, self.y)
    }
    fn abs_diff(self, other: Vector) -> Vector {
        Vector {
            x: self.x.abs_diff(other.x),
            y: self.y.abs_diff(other.y),
        }
    }
    fn sum_axes(self) -> usize {
        self.x + self.y
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
    fn unwrap_enemy(self) -> Arc<RwLock<Enemy>> {
        match self {
            Self::Player(_) => panic!("Expected enemy, found player"),
            Self::Enemy(enemy) => enemy,
        }
    }
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
#[derive(Clone, Copy, Debug)]
struct MapGenSettings {
    x: usize,
    y: usize,
    render_x: usize,
    render_y: usize,
    budget: usize,
    num_bosses: usize,
}
impl MapGenSettings {
    fn new(
        x: usize,
        y: usize,
        render_x: usize,
        render_y: usize,
        budget: usize,
        num_bosses: usize,
    ) -> MapGenSettings {
        Self {
            x,
            y,
            render_x,
            render_y,
            budget,
            num_bosses,
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
            num_bosses: usize::from_binary(binary)?,
        })
    }
}
impl ToBinary for MapGenSettings {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.x.to_binary(binary)?;
        self.y.to_binary(binary)?;
        self.render_x.to_binary(binary)?;
        self.render_y.to_binary(binary)?;
        self.budget.to_binary(binary)?;
        self.num_bosses.to_binary(binary)
    }
}
fn bell(lock: Option<&mut dyn std::io::Write>) {
    let buf = [7];
    match lock {
        Some(lock) => {
            lock.write_all(&buf).unwrap();
        }
        None => {
            std::io::stdout().write_all(&buf).unwrap();
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
// DOES include the position of what it hit
fn ray_cast(
    from: Vector,
    to: Vector,
    board: &Board,
    addr: Option<usize>,
    end_stop: bool,
    player: Vector,
) -> (Vec<Vector>, Option<Collision>) {
    let x = to.x as f64 - from.x as f64;
    let y = to.y as f64 - from.y as f64;
    let len = (x.powi(2) + y.powi(2)).sqrt();
    let delta_x = (x / len) / 2.0;
    let delta_y = (y / len) / 2.0;
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
            } else if x < to.x {
                break;
            }
            if delta_y.is_sign_positive() {
                if y > to.y {
                    break;
                }
            } else if y < to.y {
                break;
            }
        }
        precise_x += delta_x;
        precise_y += delta_y;
        x = precise_x.round() as usize;
        y = precise_y.round() as usize;
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
                collision = Some(pos.into());
                break;
            }
        }
        if let Some(enemy) = board.get_enemy(pos, addr) {
            collision = Some(enemy.into());
            break;
        }
        if pos == player {
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
    fn into_entity<'a>(self, player: &'a mut Player) -> Option<Entity<'a>> {
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
#[derive(Clone, Debug)]
struct Stats {
    // The amount of money when entering each shop
    shop_money: Vec<usize>,
    // The total amount of money gained in a run
    total_money: usize,
    // how far down you go
    depth: usize,
    // How often each item was bought
    buy_list: HashMap<ItemType, usize>,
    // What upgrades were held at death
    upgrades: Upgrades,
    // How many turns were spent in each completed level
    level_turns: Vec<usize>,
    // How many turns were spent in each shop
    shop_turns: Vec<usize>,
    // How much damage was taken in total
    damage_taken: usize,
    // How much damage was blocked in total
    damage_blocked: usize,
    // How much damage was avoided by invulnerability
    damage_invulned: usize,
    // How much damage was directly dealt by the player
    damage_dealt: usize,
    // How much health was healed
    damage_healed: usize,
    // What turn it was when the player died
    death_turn: usize,
    // How many of each spell was cast
    spell_list: HashMap<Spell, usize>,
    // How many saves were made
    num_saves: usize,
    // how many of each enemy type were killed
    kills: HashMap<u8, usize>,
    // total energy used
    energy_used: usize,
    // reward energy that was lost
    energy_wasted: usize,
    // Number of times a door was closed on a boss
    cowardice: usize,
    // What enemy type did the killing blow, or none if it was the player
    killer: Option<u8>,
    // Number of times a door was opened
    doors_opened: usize,
    // Number of times a door was closed
    doors_closed: usize,
    // Number of attacks done by you
    attacks_done: usize,
    // Number of attacks that dealt damage to you
    hits_taken: usize,
    // Number of doors opened with wasd
    doors_opened_by_walking: usize,
    // Number of enemies attacked with wasd
    enemies_hit_by_walking: usize,
}
impl Stats {
    fn new() -> Stats {
        Stats {
            shop_money: Vec::new(),
            total_money: 0,
            depth: 0,
            buy_list: HashMap::new(),
            upgrades: Upgrades::new(),
            level_turns: Vec::new(),
            shop_turns: Vec::new(),
            damage_taken: 0,
            damage_blocked: 0,
            damage_invulned: 0,
            damage_dealt: 0,
            damage_healed: 0,
            death_turn: 0,
            spell_list: HashMap::new(),
            num_saves: 0,
            kills: HashMap::new(),
            energy_used: 0,
            energy_wasted: 0,
            cowardice: 0,
            killer: None,
            doors_opened: 0,
            doors_closed: 0,
            attacks_done: 0,
            hits_taken: 0,
            doors_opened_by_walking: 0,
            enemies_hit_by_walking: 0,
        }
    }
    fn collect_death(&mut self, state: &State) {
        self.depth = state.level;
        self.upgrades = state.player.upgrades;
        self.death_turn = state.turn;
        self.killer = state
            .player
            .killer
            .map(|im_too_tired_to_come_up_with_a_good_variable_name| {
                im_too_tired_to_come_up_with_a_good_variable_name.1
            })
            .flatten();
    }
    fn add_item(&mut self, item: ItemType) {
        self.buy_list
            .insert(item, self.buy_list.get(&item).unwrap_or(&0) + 1);
    }
    fn add_spell(&mut self, spell: Spell) {
        self.spell_list
            .insert(spell, self.spell_list.get(&spell).unwrap_or(&0) + 1);
    }
    fn add_kill(&mut self, variant: enemy::Variant) {
        let key = variant.to_key();
        let prev = self.kills.get(&key).unwrap_or(&0);
        self.kills.insert(key, prev + 1);
    }
    fn list_kills(&self) {
        for (key, kills) in self.kills.iter() {
            println!("{}: {kills}", enemy::Variant::from_key(*key).kill_name());
        }
        println!("");
    }
    fn list_killer(&self) {
        println!(
            "{}",
            self.killer
                .map(|key| enemy::Variant::from_key(key).kill_name())
                .unwrap_or("Yourself")
        )
    }
}
impl FromBinary for Stats {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Stats {
            shop_money: Vec::from_binary(binary)?,
            total_money: usize::from_binary(binary)?,
            depth: usize::from_binary(binary)?,
            buy_list: HashMap::from_binary(binary)?,
            upgrades: Upgrades::from_binary(binary)?,
            level_turns: Vec::from_binary(binary)?,
            shop_turns: Vec::from_binary(binary)?,
            damage_taken: usize::from_binary(binary)?,
            damage_blocked: usize::from_binary(binary)?,
            damage_invulned: usize::from_binary(binary)?,
            damage_dealt: usize::from_binary(binary)?,
            damage_healed: usize::from_binary(binary)?,
            death_turn: usize::from_binary(binary)?,
            spell_list: HashMap::from_binary(binary)?,
            num_saves: usize::from_binary(binary)?,
            kills: HashMap::from_binary(binary)?,
            energy_used: usize::from_binary(binary)?,
            energy_wasted: usize::from_binary(binary)?,
            cowardice: usize::from_binary(binary)?,
            killer: Option::from_binary(binary)?,
            doors_opened: usize::from_binary(binary)?,
            doors_closed: usize::from_binary(binary)?,
            attacks_done: usize::from_binary(binary)?,
            hits_taken: usize::from_binary(binary)?,
            doors_opened_by_walking: usize::from_binary(binary)?,
            enemies_hit_by_walking: usize::from_binary(binary)?,
        })
    }
}
impl ToBinary for Stats {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.shop_money.to_binary(binary)?;
        self.total_money.to_binary(binary)?;
        self.depth.to_binary(binary)?;
        self.buy_list.to_binary(binary)?;
        self.upgrades.to_binary(binary)?;
        self.level_turns.to_binary(binary)?;
        self.shop_turns.to_binary(binary)?;
        self.damage_taken.to_binary(binary)?;
        self.damage_blocked.to_binary(binary)?;
        self.damage_invulned.to_binary(binary)?;
        self.damage_dealt.to_binary(binary)?;
        self.damage_healed.to_binary(binary)?;
        self.death_turn.to_binary(binary)?;
        self.spell_list.to_binary(binary)?;
        self.num_saves.to_binary(binary)?;
        self.kills.to_binary(binary)?;
        self.energy_used.to_binary(binary)?;
        self.energy_wasted.to_binary(binary)?;
        self.cowardice.to_binary(binary)?;
        self.killer.as_ref().to_binary(binary)?;
        self.doors_opened.to_binary(binary)?;
        self.doors_closed.to_binary(binary)?;
        self.attacks_done.to_binary(binary)?;
        self.hits_taken.to_binary(binary)?;
        self.doors_opened_by_walking.to_binary(binary)?;
        self.enemies_hit_by_walking.to_binary(binary)
    }
}
fn save_stats() {
    if CHEATS.load(RELAXED) {
        return;
    }
    let mut stats_saves: Vec<Stats> = Vec::new();
    match std::fs::exists(STAT_PATH).unwrap() {
        true => {
            log!("Stats file exists, checking version");
            let mut file = std::fs::File::open(STAT_PATH).unwrap();
            if Version::from_binary(&mut file).unwrap() != SAVE_VERSION {
                log!("!!!Save version mismatch!!!");
                crossterm::queue!(
                    std::io::stdout(),
                    crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
                )
                .unwrap();
                println!(
                    "{}The save format in the stats file is different than the current\
                    save format, if you leave the stats file where it is, it will be\
                    deleted, I recommend moving it.\n\x1b[0mPress enter to continue",
                    Style::new().red().bold(true).underline(true).intense(true)
                );
                std::io::stdout().flush().unwrap();
                std::io::stdin().read_line(&mut String::new()).unwrap();
            }
            stats_saves = Vec::from_binary(&mut file).unwrap();
        }
        false => {
            log!("Creating new stats file")
        }
    }
    stats_saves.push(stats().clone());
    let mut file = std::fs::File::create(STAT_PATH).unwrap();
    SAVE_VERSION.to_binary(&mut file).unwrap();
    stats_saves.to_binary(&mut file).unwrap();
    log!("Saving stats");
}
fn view_stats() {
    log!("Entering stats viewer");
    let mut input = String::new();
    let mut file = std::fs::File::open(STAT_PATH).unwrap();
    assert_eq!(
        SAVE_VERSION,
        Version::from_binary(&mut file).unwrap(),
        "Invalid save format"
    );
    let stats = Vec::<Stats>::from_binary(&mut file).unwrap();
    let mut index = 0;
    macro_rules! list {
        ($field: ident, $index: ident) => {
            match $index {
                Some(index) => {
                    println!("{index}: {:?}", stats[index].$field);
                }
                None => {
                    for stat in stats.iter() {
                        println!("{:?}", stat.$field);
                    }
                }
            }
        };
        ($field: ident, $index: ident, $method: ident) => {
            match $index {
                Some(index) => {
                    print!("{index}: ");
                    stats[index].$method()
                }
                None => {
                    for stat in stats.iter() {
                        stat.$method()
                    }
                }
            }
        };
    }
    loop {
        println!("What would you like to do?");
        input.truncate(0);
        std::io::stdin().read_line(&mut input).unwrap();
        let mut split = input.trim().split(' ');
        match split.next().unwrap() {
            "help" => println!("{}", include_str!("stat_help.txt")),
            "next" => {
                if let Ok(offset) = split.next().unwrap_or("1").parse::<usize>() {
                    let new_index = index + offset;
                    if new_index < stats.len() {
                        index = new_index;
                        println!("now at {index}");
                    } else {
                        println!("{new_index} is not a valid index");
                    }
                } else {
                    println!("Expected number, found not number");
                }
            }
            "prev" => {
                if let Ok(offset) = split.next().unwrap_or("1").parse::<usize>() {
                    if offset > index {
                        println!("Attempted to go to negative index");
                    } else {
                        index -= offset;
                        println!("now at {index}");
                    }
                }
            }
            "jump" => {
                if let Some(s) = split.next() {
                    if let Ok(new_index) = s.parse() {
                        if stats.get(new_index).is_some() {
                            index = new_index;
                        } else {
                            println!("{new_index} is not a valid index");
                        }
                    } else {
                        println!("Failed to get index");
                    }
                } else {
                    println!("Expected index to jump to")
                }
            }
            "list" => match split.next() {
                Some(field) => {
                    let index = match split.next() {
                        Some(string) => match string.parse::<usize>() {
                            Ok(index) => Some(index),
                            Err(_) => {
                                eprintln!("Invalid index");
                                continue;
                            }
                        },
                        None => None,
                    };
                    match field {
                        "shop_money" => list!(shop_money, index),
                        "total_money" => list!(total_money, index),
                        "depth" => list!(depth, index),
                        "buy_list" => list!(buy_list, index),
                        "upgrades" => list!(upgrades, index),
                        "level_turns" => list!(level_turns, index),
                        "shop_turns" => list!(shop_turns, index),
                        "damage_taken" => list!(damage_taken, index),
                        "damage_blocked" => list!(damage_blocked, index),
                        "damage_invulned" => list!(damage_invulned, index),
                        "damage_dealt" => list!(damage_dealt, index),
                        "damage_healed" => list!(damage_healed, index),
                        "death_turn" => list!(death_turn, index),
                        "spell_list" => list!(spell_list, index),
                        "num_saves" => list!(num_saves, index),
                        "kills" => list!(kills, index, list_kills),
                        "energy_used" => list!(energy_used, index),
                        "energy_wasted" => list!(energy_wasted, index),
                        "cowardice" => list!(cowardice, index),
                        "killer" => list!(killer, index, list_killer),
                        "doors_opened" => list!(doors_opened, index),
                        "doors_closed" => list!(doors_closed, index),
                        "attacks_done" => list!(attacks_done, index),
                        "hits_taken" => list!(hits_taken, index),
                        "doors_opened_by_walking" => list!(doors_opened_by_walking, index),
                        "enemies_hit_by_walking" => list!(enemies_hit_by_walking, index),
                        other => println!("{other} is not a valid field"),
                    }
                }
                None => println!("{index} out of {}:\n{:#?}", stats.len() - 1, stats[index]),
            },
            "quit" => break,
            other => println!("\"{other}\" is not a valid command"),
        }
    }
}
// Need release lock so that rendering can happen
fn arrow<'a>(
    from: Vector,
    to: Vector,
    board: &mut Board,
    player: &'a mut Player,
    time: &mut std::time::Duration,
) -> Option<Entity<'a>> {
    let mut start = std::time::Instant::now();
    let (path, collision) = ray_cast(from, to, board, None, false, player.pos);
    let bounds = board.get_render_bounds(player);
    // Visuals
    for pos in path.iter() {
        if !board.is_visible(*pos, bounds.clone(), player.effects.full_vis.is_active()) {
            continue;
        }
        let special = board.add_special(board::Special::new(*pos, '●', None));
        board.smart_render(player);
        drop(special);
        *time += start.elapsed();
        proj_delay();
        start = std::time::Instant::now();
    }
    // Returning the hit
    *time += start.elapsed();
    collision
        .map(|collision| collision.into_entity(player))
        .flatten()
}
