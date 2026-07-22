// Modules
mod board;
mod enemy;
mod input;
mod math;
mod player;
mod random;
mod state;

use board::AxisLength;
use input::Input;
use input::normalize;
use input::weirdify;
use math::Vector;
use math::Zone;

fn main() {
    abes_nice_things::set_log_path("log").expect("Failed to set log path");
    if let Err(error) = std::panic::catch_unwind(run) {
        // Panic handling
        let _ = normalize();

        std::panic::panic_any(error)
    }
}
fn run() {
    let (desired_width, desired_height) = calc_desired_dimensions();
    let mut state = state::State::new(
        board::map_gen::generate(AxisLength::Full, Vector::new(desired_width, desired_height))
            .unwrap(),
        player::Player::new(Vector::new(1, 1)),
    );

    weirdify().unwrap();
    loop {
        state.render();
        match Input::get() {
            Input::Walk(direction) => {
                player::Player::handle_walk_input(&mut state, direction);
            }
            Input::MoveSelector(direction) => {
                player::Player::handle_move_selector_input(&mut state, direction);
            }
            Input::ChangeRenderTarget => {
                player::Player::handle_change_render_target_input(&mut state);
            }
            Input::Space => {
                board::Board::pathfind(&mut state);
            }
            Input::Select => {
                if state.board.count_enemies() == 0 {
                    state.board.add_enemy(enemy::Enemy::new(
                        &enemy::dummy::VTABLE,
                        state.player.selector,
                    ));
                } else {
                    state
                        .board
                        .get_enemy_mut(board::EnemyID(0))
                        .as_mut()
                        .unwrap()
                        .move_target = Some(state.player.selector);
                }
            }
        }
    }
    //normalize().unwrap();
}
/// Calculates the desired width, height for the viewport. It gets the terminal's size then
/// subtracts the areas needed for other parts of the ui. If the resulting viewport would be too
/// small then it panics.
///
/// When using this to create a [Zone] for the viewport, remember to subtract 1 from the width and
/// height first because [Zone]s are inclusive.
fn calc_desired_dimensions() -> (usize, usize) {
    let (mut width, mut height) = get_terminal_size();

    // Viewport border
    width -= 1;
    height -= 1;

    // bars
    height -= 5;

    // Right column
    width -= 25;

    // validity checks
    if width < 20 {
        panic!("Terminal is under width")
    }
    if height < 10 {
        panic!("terminal is under height")
    }
    (width, height)
}
enum MapObject {
    Player,
    Enemy(board::EnemyID),
}
/// Gets the size of the terminal in width, height.
///
/// This takes about 10ms independant of whether it is release or debug.
fn get_terminal_size() -> (usize, usize) {
    // These get the width and height respectively, the reason why they have to inherit stderr is
    // because they ask stderr what size it is
    (
        String::from_utf8(
            std::process::Command::new("tput")
                .arg("cols")
                .stderr(std::process::Stdio::inherit())
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .parse()
        .unwrap_or(113),
        String::from_utf8(
            std::process::Command::new("tput")
                .arg("lines")
                .stderr(std::process::Stdio::inherit())
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .parse()
        .unwrap_or(35),
    )
}
#[cfg(debug_assertions)]
fn test_random() {
    let sample_size = 10000000;
    let mut average = 0.0;
    let start = std::time::Instant::now();
    let mut sections = [0, 0, 0, 0, 0]; // 0.5-6 6-7 7-8 8-9 9-1.0
    for _ in 0..sample_size {
        let random = random::random();
        if random < 0.6 {
            sections[0] += 1;
        } else if random < 0.7 {
            sections[1] += 1;
        } else if random < 0.8 {
            sections[2] += 1;
        } else if random < 0.9 {
            sections[3] += 1;
        } else {
            sections[4] += 1;
        }
        average += random / sample_size as f64;
    }
    let elapsed = start.elapsed();
    println!("Sample size: {sample_size}");
    println!("average: {average}");
    println!("Number in range 0.5-0.6: {}", sections[0]);
    println!("Number in range 0.6-0.7: {}", sections[1]);
    println!("Number in range 0.7-0.8: {}", sections[2]);
    println!("Number in range 0.8-0.9: {}", sections[3]);
    println!("Number in range 0.9-1.0: {}", sections[4]);
    println!("Time taken: {} seconds", elapsed.as_secs_f32());
    println!(
        "Average time: {} nano seconds",
        elapsed.as_nanos() / sample_size
    );
    let options = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut picks = [0; 10];
    let sample_size = 1000000000;
    let start = std::time::Instant::now();
    for _ in 0..sample_size {
        picks[*random::pick(&options)] += 1;
    }
    let elapsed = start.elapsed();
    for (slot, num_picks) in picks.iter().enumerate() {
        println!("Slot {slot} was picked {num_picks} times");
    }
    println!("Sample size: {sample_size}");
    println!("Time taken: {} seconds", elapsed.as_secs_f32());
    println!(
        "Average time: {} nano seconds",
        elapsed.as_nanos() / sample_size
    );
}
