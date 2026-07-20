use super::AxisLength;
use super::Board;
use super::Tile;
use super::convert_z_order_index;
use crate::Vector;
use crate::Zone;
use crate::random::Random;
use anyhow::Result;

// Start with a box defined by the axis length.
// Recursively divide the box into two in a random axis (prefering more square) and at a random
// point along that axis in increments of 4 tiles. There is a minimum size at a room's axis can be
// in order for it to be subdivided (if either axis is too small then it does not) and at any point
// past the 4th round of divisions, subdivisions can stop with the chance increasing as the room
// gets smaller.
//
// adjacent rooms get 2 tile wide connected doors connecting them at the midpoint in the shared
// section of wall

// Because all of this is happening in a different thread, we do not need to care about performance

pub fn generate(axis_length: AxisLength, desired_viewport: Vector<usize>) -> Result<Board> {
    let mut rooms = Vec::new();
    rooms.push(Room {
        bounds: Zone::from_vectors(
            Vector::ZERO,
            Vector::new(axis_length.to_inner() - 1, axis_length.to_inner() - 1),
        ),
        children: None,
    });
    Room::subdivide(
        &mut rooms,
        0,
        0,
        (axis_length.to_inner() as f64).powf(1.5).cbrt(),
    );
    let mut tiles = Board::create_blank_tile_array(axis_length)?;
    Room::place_walls(&mut rooms, 0, axis_length, &mut tiles)?;

    let mut board = Board::new(axis_length, desired_viewport)?;
    board.tiles = tiles;
    Ok(board)
}
struct Room {
    /// These bounds include the walls meaning that there will be overlapping edges of adjacent
    /// rooms
    bounds: Zone<usize>,
    /// If this rooms has children, then it is the indices of those children
    children: Option<[usize; 2]>,
}
impl Room {
    const MINIMUM_AXIS: usize = 12; // 3 increments of 4
    const MINIMUM_STOP_DEPTH: usize = 3;
    fn subdivide(rooms: &mut Vec<Room>, index: usize, depth: usize, max_early_stop: f64) {
        let smallest_axis_length = rooms[index]
            .bounds
            .height()
            .min(rooms[index].bounds.width());
        let smallest_axis = if rooms[index].bounds.height() > rooms[index].bounds.width() {
            // taller than wide
            // width is small
            Axis::Horizontal
        } else {
            Axis::Vertical
        };

        // Room is too small to divide
        if smallest_axis_length <= Room::MINIMUM_AXIS {
            return;
        }

        // Random chance of stopping division based on size
        if depth > Room::MINIMUM_STOP_DEPTH
            && (smallest_axis_length as f64)
                < (crate::random::random() - 0.5) * 2.0 * max_early_stop
        {
            return;
        }

        // We are going to subdivide

        // deciding the axis for division
        // 1 in 4 chance of dividing the smallest axis instead of the bigger one
        let division_axis = if (u8::random() & 0b11) == 0 {
            smallest_axis
        } else {
            !smallest_axis
        };

        // Getting the bounds of the to be divided axis
        let (range_start, range_end) = match division_axis {
            Axis::Horizontal => (rooms[index].bounds.left(), rooms[index].bounds.right()),
            Axis::Vertical => (rooms[index].bounds.top(), rooms[index].bounds.bottom()),
        };

        // Picking division position
        let split_point = ((((crate::random::random() + crate::random::random()) / 2.0) - 0.5)
            * 2.0
            * (range_end - range_start - 8) as f64) as usize
            + range_start
            + 4;

        // Creating children

        // Top left child
        rooms.push(Room {
            bounds: Zone::from_vectors(
                rooms[index].bounds.top_left(),
                match division_axis {
                    Axis::Horizontal => Vector::new(split_point, rooms[index].bounds.bottom()),
                    Axis::Vertical => Vector::new(rooms[index].bounds.right(), split_point),
                },
            ),
            children: None,
        });
        // Bottom right child
        rooms.push(Room {
            bounds: Zone::from_vectors(
                match division_axis {
                    Axis::Horizontal => Vector::new(split_point, rooms[index].bounds.top()),
                    Axis::Vertical => Vector::new(rooms[index].bounds.left(), split_point),
                },
                rooms[index].bounds.bottom_right(),
            ),
            children: None,
        });

        // Saving children indices
        rooms[index].children = Some([rooms.len() - 2, rooms.len() - 1]);

        // Recursing deeper
        Room::subdivide(
            rooms,
            rooms[index].children.unwrap()[0],
            depth + 1,
            max_early_stop,
        );
        Room::subdivide(
            rooms,
            rooms[index].children.unwrap()[1],
            depth + 1,
            max_early_stop,
        );
    }
    fn place_walls(
        rooms: &mut Vec<Room>,
        index: usize,
        axis_length: AxisLength,
        tiles: &mut Vec<Option<Tile>>,
    ) -> Result<()> {
        // Recursing deeper
        if let Some(children) = rooms[index].children {
            Room::place_walls(rooms, children[0], axis_length, tiles)?;
            Room::place_walls(rooms, children[1], axis_length, tiles)?;
            return Ok(());
        }

        // We are at a leaf

        // Horizontal walls
        for x in rooms[index].bounds.left()..=rooms[index].bounds.right() {
            tiles[convert_z_order_index(Vector::new(x, rooms[index].bounds.top()), axis_length)?] =
                Some(Tile::Wall);
            tiles[convert_z_order_index(
                Vector::new(x, rooms[index].bounds.bottom()),
                axis_length,
            )?] = Some(Tile::Wall);
        }
        // Vertical walls
        for y in rooms[index].bounds.top()..=rooms[index].bounds.bottom() {
            tiles
                [convert_z_order_index(Vector::new(rooms[index].bounds.left(), y), axis_length)?] =
                Some(Tile::Wall);
            tiles[convert_z_order_index(
                Vector::new(rooms[index].bounds.right(), y),
                axis_length,
            )?] = Some(Tile::Wall);
        }
        Ok(())
    }
}
enum Axis {
    Horizontal,
    Vertical,
}
impl std::ops::Not for Axis {
    type Output = Axis;
    fn not(self) -> Self::Output {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }
}
