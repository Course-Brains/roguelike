// Modules
mod board;
mod state;
mod vector;

use vector::Vector;
use vector::Zone;

fn main() {
    abes_nice_things::set_log_path("log").expect("Failed to set log path");
    let board = board::Board::new(6).unwrap(); // 64x64
    let zone = Zone::from_vectors(Vector::ZERO, Vector::new(59, 29));
    board.render_tiles(zone);
}
