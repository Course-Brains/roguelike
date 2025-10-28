// When I do add spells, add a system for random unidentifiable buffs that get determined at the
// start, with one of them being the ability to do other actions while casting

// The format version of the save data, different versions are incompatible and require a restart
// of the save, but the version will only change on releases, so if the user is not going by
// release, then they could end up with two incompatible save files.
const SAVE_VERSION: Version = 16;
mod player;
use player::Player;
mod board;
use board::{Board, Piece};
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
mod settings;
use settings::{Difficulty, Settings};
mod stats;
use stats::*;
mod vector;
use vector::*;
mod state;
use state::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, LazyLock, Mutex, RwLock};

use abes_nice_things::style::{Color, Style};
use abes_nice_things::{FromBinary, ToBinary};

// Convenience constant
const RELAXED: Ordering = Ordering::Relaxed;

// The number that the turn count is divided by to get the budget
const BUDGET_DIVISOR: usize = 5;
// The number of bosses in each level starting at the third level
const NUM_BOSSES: usize = 5;
// The budget given per layer (layer * this)
const BUDGET_PER_LAYER: usize = 100;

static LOG: Mutex<Option<File>> = Mutex::new(None);
static STATS: LazyLock<Mutex<Stats>> = LazyLock::new(|| Mutex::new(Stats::new()));
fn stats<'a>() -> std::sync::MutexGuard<'a, Stats> {
    STATS.try_lock().unwrap()
}
// Whether or not the console was used
static CHEATS: AtomicBool = AtomicBool::new(false);
mod bench {
    use std::fs::File;
    use std::sync::{LazyLock, RwLock, RwLockWriteGuard, atomic::AtomicBool};
    // Whether or not to esspecially log timings
    pub static BENCHMARK: AtomicBool = AtomicBool::new(false);
    pub static USED: AtomicBool = AtomicBool::new(false);
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
    pub fn initialize_files() {
        for bench in BENCHES.iter() {
            LazyLock::force(bench);
        }
    }
}
fn enable_benchmark() {
    bench::BENCHMARK.store(true, Ordering::SeqCst);
    if bench::USED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        bench::initialize_files()
    }
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
    ($($val:tt)*) => {
        compile_error!("Someone forgot to replace the placeholder value!")
    };
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! debug_only {
    ($($val:tt)*) => {
        $($val)*
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
    println!("{}", feedback());
    std::io::stdout().flush().unwrap();
}
static SETTINGS: std::sync::LazyLock<Settings> = std::sync::LazyLock::new(Settings::get_from_file);
static LAYER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
fn layer() -> usize {
    LAYER.load(RELAXED)
}

static INPUT_SYSTEM: std::sync::LazyLock<(
    std::sync::mpsc::Sender<CommandInput>,
    Mutex<std::sync::mpsc::Receiver<CommandInput>>,
)> = std::sync::LazyLock::new(|| {
    let (send, recv) = std::sync::mpsc::channel();
    (send, Mutex::new(recv))
});
fn initialize_stdin_listener() -> std::thread::Thread {
    std::thread::spawn(|| {
        let send = INPUT_SYSTEM.0.clone();
        loop {
            std::thread::park();
            send.send(CommandInput::Input(Input::get())).unwrap();
        }
    })
    .thread()
    .clone()
}
// Sends the current upgrades to the shop generator so that it can only show non-redundant upgrades
type ShopAvailabilityData = Vec<upgrades::UpgradeType>;
static SHOP_AVAILABILITY: std::sync::LazyLock<
    Mutex<(
        std::sync::mpsc::Sender<ShopAvailabilityData>,
        std::sync::mpsc::Receiver<ShopAvailabilityData>,
    )>,
> = std::sync::LazyLock::new(|| Mutex::new(std::sync::mpsc::channel()));

enum CommandInput {
    Input(input::Input),
    Command(commands::Command),
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
    std::sync::LazyLock::force(&SETTINGS);
    let mut args = std::env::args();
    let mut counting = false;
    let mut testing = false;
    let mut initial_board = {
        match std::fs::exists("stats").unwrap() || SETTINGS.difficulty() >= Difficulty::Hard {
            true => InitialBoard::Normal,
            false => InitialBoard::Tutorial,
        }
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" | "-s" => {
                let new_index = args.next().unwrap().parse().unwrap();
                CHEATS.store(true, RELAXED);
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
                initial_board = InitialBoard::Empty;
            }
            "settings" => {
                let _weirdifier = Weirdifier::new();
                log!("Openning settings editor");
                SETTINGS.clone().editor();
                return;
            }
            "--no-stats" => CHEATS.store(true, Ordering::Relaxed),
            "--port" | "-p" => {
                let new_port = args.next().unwrap().parse().unwrap();
                log!("Setting console port to {new_port}");
                commands::PORT.store(new_port, RELAXED);
            }
            _ => {}
        }
    }
    if testing {
        generate(MapGenSettings::new(
            151,
            151,
            45,
            15,
            75,
            1,
            State::level_0_highest_tier(),
        ))
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
            let board = generate(MapGenSettings::new(
                151,
                151,
                45,
                15,
                State::level_0_budget(),
                1,
                State::level_0_highest_tier(),
            ))
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

    /*crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )
    .unwrap();*/
    let save_file = std::fs::exists(PATH).unwrap();
    let mut state = match save_file {
        // There is a save file
        true => {
            match State::from_binary(&mut std::fs::File::open(PATH).unwrap()) {
                Ok(state) => state,
                Err(error) => {
                    if error.kind() == std::io::ErrorKind::Other {
                        // The player changed the difficulty between save and load time
                        return;
                    } else {
                        panic!("{error}")
                    }
                }
            }
        }
        // there is not a save file
        false => State::new(initial_board),
    };
    state
        .shop_sender
        .send(state.player.upgrades.get_available())
        .unwrap();
    // discourage save scumming by making it so that if it closes non-properly then the file is
    // gone
    if SETTINGS.difficulty() != Difficulty::Easy {
        let _ = std::fs::remove_file(PATH);
    }
    generator::DO_DELAY.store(true, Ordering::SeqCst);
    state.next_map = generate(state.next_map_settings);

    let mut command_tx = commands::listen(INPUT_SYSTEM.0.clone());
    state.board.flood(state.player.pos);
    let _weirdifier = Weirdifier::new();
    state.render();
    let input_thread = initialize_stdin_listener();
    let input_lock = INPUT_SYSTEM.1.try_lock().unwrap();
    let mut got_input = true;
    loop {
        if state.player.is_dead() {
            stats().collect_death(&state);
            let show_stats = Player::handle_death(&state);
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
        if got_input {
            input_thread.unpark();
            got_input = false
        }
        match input_lock.recv().unwrap() {
            CommandInput::Input(input) => {
                got_input = true;
                match input {
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
                            if BONUS_NO_ENERGY.load(RELAXED) {
                                set_feedback("Was going faster worth it?".to_string());
                                bell(Some(&mut std::io::stdout()));
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
                            } else if state.is_valid_move(direction) {
                                state.player.do_move(direction, &mut state.board);
                                state.increment()
                            } else if state
                                .board
                                .get_enemy(state.player.pos + direction, None)
                                .is_some()
                                && SETTINGS.kick_enemies()
                            {
                                state.attack_enemy(
                                    state.player.pos + direction,
                                    false,
                                    false,
                                    true,
                                );
                                state.increment();
                            } else if !SETTINGS.kick_doors() {
                                // Doing it like this because can't do && on if let
                            } else if let Some(board::Piece::Door(door)) =
                                state.board[state.player.pos + direction]
                                && !door.open
                            {
                                state.open_door(state.player.pos + direction, true);
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
                            stats().energy_used += 1;
                            state.player.was_hit = false;
                            state.player.blocking = true;
                            state.increment();
                            if state.player.was_hit {
                                if BONUS_NO_ENERGY.load(RELAXED) {
                                    set_feedback("Did you really need to block that?".to_string());
                                    bell(Some(&mut std::io::stdout()));
                                }
                                BONUS_NO_ENERGY.store(false, RELAXED);
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
                        if let Some(Piece::Door(door)) = &mut state.board[state.player.selector] {
                            if door.open {
                                door.open = false;
                                state.board.flood(state.player.pos);
                                state.increment();
                            } else {
                                state.open_door(state.player.selector, false);
                            }
                        }
                    }
                    Input::Use(index) => {
                        debug_assert!(index < 7);
                        match state.player.right_column {
                            player::RightColumn::Items => {
                                if let Some(item) = state.player.items[index - 1] {
                                    if item.enact(&mut state) {
                                        state.player.items[index - 1] = None;
                                        state.increment();
                                    }
                                } else {
                                    bell(None);
                                }
                            }
                            player::RightColumn::Spells => {
                                if let Some(spell) = state.player.known_spells[index - 1] {
                                    if spell.energy_needed() <= state.player.energy {
                                        let mut succeeded = false;
                                        // Valid cast
                                        match spell {
                                            Spell::Normal(spell) => {
                                                for _ in 0..spell.cast_time() {
                                                    state.increment();
                                                }
                                                let origin = Some(state.player.pos);
                                                let aim = Some(state.player.selector);
                                                spell.cast(
                                                    None,
                                                    &mut state.player,
                                                    &mut state.board,
                                                    origin,
                                                    aim,
                                                    None,
                                                );
                                                succeeded = true;
                                            }
                                            Spell::Contact(spell) => {
                                                if !state
                                                    .player
                                                    .pos
                                                    .is_near(state.player.selector, 2)
                                                {
                                                    set_feedback(
                                                        "You can only cast contact spells when \
                                                    within melee range of the target"
                                                            .to_string(),
                                                    );
                                                    draw_feedback();
                                                    bell(None);
                                                    continue;
                                                }
                                                if let Some(target) = state
                                                    .board
                                                    .get_enemy(state.player.selector, None)
                                                {
                                                    for _ in 0..spell.cast_time() {
                                                        state.increment();
                                                    }
                                                    spell.cast(
                                                        Entity::Enemy(target),
                                                        Entity::Player(&mut state.player),
                                                    );
                                                    succeeded = true;
                                                } else {
                                                    set_feedback(
                                                        "Contact spells require a target"
                                                            .to_string(),
                                                    );
                                                    draw_feedback();
                                                    bell(None);
                                                }
                                            }
                                        }
                                        if succeeded {
                                            state.player.energy -= spell.energy_needed();
                                            stats().energy_used += spell.energy_needed();
                                            if !BONUS_NO_ENERGY.swap(true, RELAXED) {
                                                set_feedback(
                                                    "Did you really need to cast that?".to_string(),
                                                );
                                                bell(Some(&mut std::io::stdout()));
                                            }
                                            state.render();
                                        }
                                    } else {
                                        set_feedback("Insufficient energy".to_string());
                                        draw_feedback();
                                        bell(None);
                                    }
                                } else {
                                    set_feedback(debug_only!("invalid spell index".to_string()));
                                    draw_feedback();
                                    bell(None);
                                }
                            }
                        }
                    }
                    Input::Convert => {
                        if state.player.upgrades.precise_convert {
                            if state.player.energy > 0 {
                                if SETTINGS.difficulty() == Difficulty::Easy {
                                    state.player.give_money(2);
                                } else {
                                    state.player.give_money(1);
                                }
                                state.player.energy -= 1;
                            }
                        } else {
                            if SETTINGS.difficulty() == Difficulty::Easy {
                                state.player.give_money(2 * state.player.energy);
                            } else {
                                state.player.give_money(state.player.energy);
                            }
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
                        if SETTINGS.fast_mode() {
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
                    }
                    Input::ClearFeedback => {
                        *feedback() = String::new();
                        state.render();
                    }
                    Input::Memorize => {
                        state.player.memory = Some(state.player.selector);
                        set_feedback(format!(
                            "You have memorized the position: {}",
                            state.player.selector
                        ));
                        stats().times_memorized += 1;
                        state.render();
                    }
                    Input::Remember => {
                        set_feedback(match state.player.memory {
                            Some(memory) => {
                                stats().times_remembered += 1;
                                format!("You remember the position {memory}")
                            }
                            None => "You have made no mental notes in this place.".to_string(),
                        });
                        state.render();
                    }
                    Input::AutoMove => {
                        log!("!Starting auto move!");
                        if state.board.is_reachable(state.player.selector)
                            && SETTINGS.auto_move()
                            && (state.board.seen[state.board.to_index(state.player.selector)]
                                || state.player.upgrades.map)
                        {
                            // Janky hack to get backtrace data to the target
                            state.board.enemies.push(Arc::new(RwLock::new(Enemy::new(
                                state.player.selector,
                                enemy::Variant::basic(),
                            ))));
                            state.board.generate_nav_data(
                                state.player.pos,
                                false,
                                None,
                                &mut state.player,
                            );
                            state.board.enemies.pop();
                            let directions = state
                                .board
                                .get_directions(state.player.selector, state.player.pos);
                            let send = &INPUT_SYSTEM.0;
                            for direction in directions.into_iter() {
                                send.send(CommandInput::Input(Input::Wasd(direction, false)))
                                    .unwrap();
                            }
                        }
                    }
                    Input::ChangeRightColumn => {
                        state.player.right_column.increment();
                        state.render();
                    }
                }
            }
            CommandInput::Command(command) => {
                command.enact(&mut state, &mut command_tx);
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
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EnableLineWrap,
            crossterm::cursor::Show
        )
        .unwrap()
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
    max_enemy_tier: Option<usize>,
}
impl MapGenSettings {
    fn new(
        x: usize,
        y: usize,
        render_x: usize,
        render_y: usize,
        budget: usize,
        num_bosses: usize,
        max_enemy_tier: Option<usize>,
    ) -> MapGenSettings {
        Self {
            x,
            y,
            render_x,
            render_y,
            budget,
            num_bosses,
            max_enemy_tier,
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
            max_enemy_tier: Option::from_binary(binary)?,
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
        self.num_bosses.to_binary(binary)?;
        self.max_enemy_tier.as_ref().to_binary(binary)
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
        if let Some(piece) = &board[pos]
            && piece.projectile_collision()
        {
            collision = Some(pos.into());
            break;
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
    collision.and_then(|collision| collision.into_entity(player))
}
fn should_do_tutorial() -> bool {
    if SETTINGS.difficulty() >= Difficulty::Hard {
        false
    } else {
        !std::fs::exists("stats").unwrap()
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InitialBoard {
    Normal,
    Empty,
    Tutorial,
}
