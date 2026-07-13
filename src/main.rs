// Modules
mod board;
mod random;
mod state;
mod vector;

use vector::Vector;
use vector::Zone;

fn main() {
    abes_nice_things::set_log_path("log").expect("Failed to set log path");

    let sample_size = 10000000;
    let mut average = 0.0;
    let start = std::time::Instant::now();
    for _ in 0..sample_size {
        average += random::random() / sample_size as f64;
    }
    let elapsed = start.elapsed();
    println!("average: {average}");
    println!("Time taken: {} seconds", elapsed.as_secs_f32());
    println!(
        "Average time: {} nano seconds",
        elapsed.as_nanos() / sample_size
    );

    /*let board = board::Board::new(6).unwrap(); // 64x64
    let zone = Zone::from_vectors(Vector::ZERO, Vector::new(59, 29));
    board.render_tiles(zone);*/
}
