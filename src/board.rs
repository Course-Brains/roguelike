mod tile;

use crate::Vector;
use crate::Zone;
use abes_nice_things::PrimAs;
use anyhow::{Context, Result, bail};
use tile::Tile;
/// This contains all data which is tied to the specific map, which is everything that does not
/// carry over between maps.
///
/// - the [Tile]s
///
pub struct Board {
    /// Implemented with z order traversal, meaning that it MUST have axis lengths equal to an
    /// exponent of 2 and MUST be a square.
    ///
    /// Do NOT change the length of this array. Seriously, DON'T.
    tiles: Vec<Option<Tile>>,
    /// The length of each axis of the map
    axis_length: usize,
}
impl Board {
    /// Creates a blank board which is not populated by tile objects or map objects
    pub fn new(tile_axis_bits: usize) -> Result<Board> {
        Ok(Board {
            tiles: Board::create_blank_tile_array(tile_axis_bits)?,
            axis_length: 1 << tile_axis_bits,
        })
    }
}

// RENDERING
impl Board {
    /// Zeros the cursor and draws the tiles onto the screen, this is the first layer of rendering
    pub fn render_tiles(&self, view: Zone<usize>) {
        // Putting the cursor in the top corner
        print!("\x1b[H");
        for (position, last) in view.scanlines() {
            if let Some(tile) = self[position] {
                let (ch, style) = tile.render(self, position);
                match style {
                    Some(style) => print!("{style}{ch}\x1b[0m"),
                    None => print!("{ch}"),
                }
            } else {
                if last {
                    print!(
                        "{} \x1b[0m",
                        abes_nice_things::Style::new().background_red()
                    );
                } else {
                    print!(
                        "{} \x1b[0m",
                        abes_nice_things::Style::new().background_green()
                    );
                }
            }
            if last {
                // erase until end of line
                println!("\x1b[0K");
            }
        }
        // erase from cursor to end of screen
        print!("\x1b[0J");
    }
}

// TILES
impl Board {
    /// ALWAYS ensure this matches the implementations for indexing into the tiles.
    /// This is the maximum length of each axis for the board.
    const MAX_AXIS_LENGTH: usize = 0b0011_1111_1111; // 1024
    /// The maximum number of bits in an axis of an index.
    /// This must be less than or equal to half of the length of usize
    const MAX_AXIS_BITS: usize = 10; // see above

    /// This will create a validly sized empty tile array, or return error if you tried to make one
    /// that is too big.
    fn create_blank_tile_array(axis_bits: usize) -> Result<Vec<Option<Tile>>> {
        // Validation
        if axis_bits > Board::MAX_AXIS_BITS {
            bail!("Attempted to create an oversized tile array:
                maxiumum bits per axis is {} but attempted to create an array with {axis_bits} bits", Board::MAX_AXIS_BITS);
        }

        // Vec length calculation
        let axis_length = 1 << axis_bits;
        let length = axis_length * axis_length;

        // Vec creation
        Ok(vec![const { None }; length])
    }
    /// This will attempt to get the tile at a position and will return None if it is out of bounds
    /// in any direction (yes this does work with negatives).
    pub fn try_get_tile<T: abes_nice_things::Number>(
        &self,
        position: Vector<T>,
    ) -> Option<&Option<Tile>> {
        // Negative position
        if position.x < T::prim_from(0) || position.y < T::prim_from(0) {
            return None;
        }
        // Out of bounds
        if position.x > T::prim_from(self.axis_length)
            || position.y > T::prim_from(self.axis_length)
        {
            return None;
        }
        Some(&self[position.prim_as()])
    }
}

// INDEXING
impl std::ops::Index<Vector<usize>> for Board {
    type Output = Option<Tile>;
    fn index(&self, index: Vector<usize>) -> &Self::Output {
        let true_index = convert_z_order_index(index, self.axis_length)
            .context("While tile indexing")
            .unwrap();
        &self.tiles[true_index]
    }
}
impl std::ops::IndexMut<Vector<usize>> for Board {
    fn index_mut(&mut self, index: Vector<usize>) -> &mut Self::Output {
        let true_index = convert_z_order_index(index, self.axis_length)
            .context("While tile indexing")
            .unwrap();
        &mut self.tiles[true_index]
    }
}
fn convert_z_order_index(index: Vector<usize>, axis_length: usize) -> Result<usize> {
    // Checking validity
    if index.x > axis_length || index.y > axis_length {
        bail!(
            "Could not generate z order index because logical index was out of bounds.
            ({},{}) is out of bounds for z order array with axis length {axis_length}",
            index.x,
            index.y
        );
    }
    debug_assert!(axis_length < Board::MAX_AXIS_LENGTH);

    // They call me Jacque the Zipper
    let mut true_index = 0;
    for bit in 0..Board::MAX_AXIS_BITS {
        true_index |= (index.x & (1 << bit)) << bit;
        true_index |= (index.y & (1 << bit)) << (bit + 1);
    }
    // 0 1 0 1 0 1 0 1
    // 7 6 5 4 3 2 1 0
    // 3 3 2 2 1 1 0 0

    Ok(true_index)
}
#[cfg(test)]
#[test]
fn validate_z_order() {
    let array = [(); 64 * 64]; // 64 x 64

    for x in 0..64_usize {
        for y in 0..64_usize {
            println!("Getting {x}, {y}");
            array[convert_z_order_index(Vector::new(x, y), 64).unwrap()];
        }
    }
}
#[cfg(test)]
#[test]
fn validate_tile_indexing() {
    let board = Board::new(6).unwrap(); // 64 x 64

    for x in 0..64_usize {
        for y in 0..64_usize {
            //println!("Getting {x}, {y}");
            board[Vector::new(x, y)];
        }
    }
}
