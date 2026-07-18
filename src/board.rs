mod axis_length;
pub mod tile;
pub use axis_length::AxisLength;

use crate::Vector;
use crate::Zone;
use crate::enemy::Enemy;
use crate::state::State;
use abes_nice_things::Number;
use abes_nice_things::PrimAs;
use abes_nice_things::log;
use anyhow::{Context, Result, bail};
use std::io::Write;
use tile::Tile;
/// This contains all data which is tied to the specific map, which is everything that does not
/// carry over between maps.
///
/// - the [Tile]s
/// - the [AxisLength]
/// - the viewport's size
///
/// It is VERY important to note that the [Tile] [Vec] MUST not change length after the board is
/// created and the [AxisLength] MUST not change either. They have to be tied to each other which
/// is why you cannot change either.
pub struct Board {
    /// Implemented with z order traversal, meaning that it MUST have axis lengths equal to an
    /// exponent of 2 and MUST be a square.
    ///
    /// Do NOT change the length of this array. Seriously, DON'T.
    tiles: Vec<Option<Tile>>,
    /// The length of each axis of the map
    axis_length: AxisLength,
    /// The size of the viewport, the center will tend towards the top left
    viewport_size: Vector<usize>,
    /// Elements in this array MUST never be removed or reordered, if an enemy dies, its entry
    /// must change to None instead of removing the entry. This is to preserve index validity even
    /// if the enemy at that index dies.
    ///
    /// Do not interact with this directly, there are functions which are defined ways which will
    /// not change out from under you.
    pub enemies: Vec<Option<Enemy>>,
    /// The number of turns spent on this map
    local_turns: usize,
}
impl Board {
    /// Creates a blank board which is not populated by tile objects or map objects and is
    /// therefore not valid
    pub fn new(axis_length: AxisLength, desired_viewport: Vector<usize>) -> Result<Board> {
        Ok(Board {
            tiles: Board::create_blank_tile_array(axis_length)?,
            axis_length,
            viewport_size: desired_viewport
                .min(Vector::new(axis_length.to_inner(), axis_length.to_inner())),
            enemies: Vec::new(),
            local_turns: 0,
        })
    }
    pub fn axis_length(&self) -> AxisLength {
        self.axis_length
    }
}

// RENDERING
impl Board {
    const VIEWPORT_BORDER_RIGHT: char = '│';
    const VIEWPORT_BORDER_BOTTOM: char = '─';
    const VIEWPORT_BORDER_CORNER: char = '╯';
    pub fn calculate_viewport(&self, mut center: Vector<usize>) -> Zone<usize> {
        // We can assume that there will be no situation in which we are against opposing walls and
        // that the viewport will not be bigger than the map in either axis
        //
        // Center will tend toward the top left
        let distance_left = self.viewport_size.x / 2;
        let distance_right = self.viewport_size.x - distance_left;
        let distance_up = self.viewport_size.y / 2;
        let distance_down = self.viewport_size.y - distance_up;

        center
            .x
            .max_assign(distance_left)
            .min_assign(self.axis_length.to_inner() - distance_right);
        center
            .y
            .max_assign(distance_up)
            .min_assign(self.axis_length.to_inner() - distance_down);

        Zone::new(
            center.x - distance_left,
            center.x + distance_right - 1,
            center.y - distance_up,
            center.y + distance_down - 1,
        )
        .unwrap()
    }
    pub fn render(&self, center: Vector<usize>) {
        let viewport = self.calculate_viewport(center);

        self.render_tiles(viewport);
        self.render_enemies(viewport);
        std::io::stdout().flush().unwrap()
    }
    /// Zeros the cursor and draws the tiles onto the screen and clears the screen, this is the first layer of rendering.
    ///
    /// Additionally it draws the border of the viewport
    fn render_tiles(&self, viewport: Zone<usize>) {
        // Putting the cursor in the top corner
        print!("\x1b[H");
        for (position, last) in viewport.scanlines() {
            if let Some(tile) = self[position] {
                let (ch, style) = tile.render(self, position);
                match style {
                    Some(style) => print!("{style}{ch}\x1b[0m"),
                    None => print!("{ch}"),
                }
            } else {
                print!(" ");
            }
            if last {
                // erase until end of line and draw right border
                println!("{}\x1b[0K", Board::VIEWPORT_BORDER_RIGHT);
            }
        }
        // erase from cursor to end of screen and draw bottom of border
        print!(
            "{}{}\x1b[0J",
            Board::VIEWPORT_BORDER_BOTTOM
                .to_string()
                .repeat(viewport.width()),
            Board::VIEWPORT_BORDER_CORNER
        );
    }
    /// Moves the cursor about to draw the enemies, this is the second layer of rendering.
    fn render_enemies(&self, viewport: Zone<usize>) {
        // The weird iterator stuff ensures that we only are rendering enemies which are alive and
        // on screen on top of getting us the on screen position of that enemy
        for (position, (character, style)) in self
            .enemies
            .iter()
            .filter_map(|enemy| enemy.as_ref())
            .filter(|enemy| viewport.contains(enemy.position))
            .map(|enemy| (enemy.position - viewport.top_left(), enemy.render()))
        {
            match style {
                Some(style) => {
                    print!(
                        "\x1b[{};{}H{style}{character}\x1b[0m",
                        position.y, position.x
                    );
                }
                None => {
                    print!("\x1b[{};{}H{character}", position.y, position.x);
                }
            }
        }
    }
}

// TILES
impl Board {
    /// ALWAYS ensure this matches the implementations for indexing into the tiles.
    /// This is the maximum length of each axis for the board.
    const MAX_AXIS_LENGTH: usize = (0b1 << Board::MAX_AXIS_BITS) - 1; // 1024
    /// The maximum number of bits in an axis of an index.
    /// This must be less than or equal to half of the length of usize
    const MAX_AXIS_BITS: usize = 10; // see above

    /// This will create a validly sized empty tile array, or return error if you tried to make one
    /// that is too big.
    fn create_blank_tile_array(axis_length: AxisLength) -> Result<Vec<Option<Tile>>> {
        // Validation
        if axis_length.to_inner() > Board::MAX_AXIS_LENGTH {
            bail!("Attempted to create an oversized tile array:
                maxiumum bits per axis is {} but attempted to create an array with {axis_length} sides", Board::MAX_AXIS_BITS);
        }

        // Vec length calculation
        let axis_length = axis_length.to_inner();
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
        if position.x > T::prim_from(self.axis_length.to_inner())
            || position.y > T::prim_from(self.axis_length.to_inner())
        {
            return None;
        }
        Some(&self[position.prim_as()])
    }
}

// ENEMIES
pub struct EnemyID(usize);
impl Board {
    pub fn add_enemy(&mut self, enemy: crate::enemy::Enemy) -> EnemyID {
        self.enemies.push(Some(enemy));
        return EnemyID(self.enemies.len() - 1);
    }
    pub fn get_enemy(&self, id: EnemyID) -> &Option<Enemy> {
        &self.enemies[id.0]
    }
    pub fn get_enemy_mut(&mut self, id: EnemyID) -> &mut Option<Enemy> {
        &mut self.enemies[id.0]
    }
    pub fn run_thinkers(state: &mut State) {
        for index in 0..state.board.enemies.len() {
            if state.board.enemies[index].is_some() {
                let vtable = state.board.enemies[index].as_ref().unwrap().get_vtable();
                (vtable.think)(state, EnemyID(index));
            }
        }
    }
}

// INDEXING
impl std::ops::Index<Vector<usize>> for Board {
    type Output = Option<Tile>;
    fn index(&self, index: Vector<usize>) -> &Self::Output {
        debug_assert_eq!(
            self.tiles.len(),
            self.axis_length.to_inner() * self.axis_length.to_inner()
        );
        let true_index = convert_z_order_index(index, self.axis_length)
            .context("While tile indexing")
            .unwrap();
        &self.tiles[true_index]
    }
}
impl std::ops::IndexMut<Vector<usize>> for Board {
    fn index_mut(&mut self, index: Vector<usize>) -> &mut Self::Output {
        debug_assert_eq!(
            self.tiles.len(),
            self.axis_length.to_inner() * self.axis_length.to_inner()
        );
        let true_index = convert_z_order_index(index, self.axis_length)
            .context("While tile indexing")
            .unwrap();
        &mut self.tiles[true_index]
    }
}
fn convert_z_order_index(index: Vector<usize>, axis_length: AxisLength) -> Result<usize> {
    // Checking validity
    if index.x >= axis_length.to_inner() || index.y >= axis_length.to_inner() {
        bail!(
            "Could not generate z order index because logical index was out of bounds.\
            \n({},{}) is out of bounds for z order array with axis length {axis_length}",
            index.x,
            index.y
        );
    }
    debug_assert!(axis_length.to_inner() < Board::MAX_AXIS_LENGTH);

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
            array[convert_z_order_index(Vector::new(x, y), AxisLength::Small).unwrap()];
        }
    }
}
#[cfg(test)]
#[test]
fn validate_tile_indexing() {
    let board = Board::new(AxisLength::Small, Vector::new(0, 0)).unwrap(); // 64 x 64

    for x in 0..64_usize {
        for y in 0..64_usize {
            //println!("Getting {x}, {y}");
            board[Vector::new(x, y)];
        }
    }
}
