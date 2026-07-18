// Modules
mod board;
mod enemy;
mod input;
mod random;
mod state;
mod vector;

use board::AxisLength;
use input::Input;
use input::normalize;
use input::weirdify;
use vector::Vector;
use vector::Zone;

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
    let mut board = board::Board::new(
        AxisLength::Small,
        Vector::new(desired_width, desired_height),
    )
    .unwrap(); // 128x128
    let len = board.axis_length().to_inner() - 1;
    board[Vector::new(0, 0)] = Some(board::tile::Tile::Marker);
    board[Vector::new(len, 0)] = Some(board::tile::Tile::Marker);
    board[Vector::new(0, len)] = Some(board::tile::Tile::Marker);
    board[Vector::new(len, len)] = Some(board::tile::Tile::Marker);
    board.add_enemy(enemy::Enemy::new(&enemy::dummy::VTABLE, Vector::new(5, 5)));

    let mut position = Vector::new(5, 5);
    weirdify().unwrap();
    board.render(position);
    loop {
        if let Input::Direction(direction) = input::Input::get() {
            position += direction;
            board.render(position);
        }
    }
    normalize().unwrap();
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
    width -= 2;
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
        .unwrap(),
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
        .unwrap(),
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
