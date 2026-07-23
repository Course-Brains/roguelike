use super::AxisLength;
use super::Board;
use super::Tile;
use super::convert_z_order_index;
use crate::Vector;
use crate::Zone;
use crate::math::Axis;
use crate::random::Random;
use anyhow::Result;

// Start with a box defined by the axis length.
// Recursively divide the box into two in a random axis (prefering more square) and at a random
// point along that axis in increments of 4 tiles. There is a minimum size at a room's axis can be
// in order for it to be subdivided (if either axis is too small then it does not) and at any point
// past the 4th round of divisions, subdivisions can stop with the chance increasing as the room
// gets smaller.
//
// adjacent rooms get doors connecting them at the midpoint in the shared
// section of wall

// Because all of this is happening in a different thread, we do not need to care about
// performance*

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
    Room::create_counterparts(&mut rooms, 0, &mut board);
    Room::fill_counterpart_adjacencies(&mut board);
    Room::set_room_map(&mut board);
    validate(&board);
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
    fn create_counterparts(rooms: &mut Vec<Room>, index: usize, board: &mut Board) {
        // Recursing
        if let Some(children) = rooms[index].children {
            Room::create_counterparts(rooms, children[0], board);
            Room::create_counterparts(rooms, children[1], board);
            return;
        }

        // We are at a leaf
        board.add_room(super::Room::new(rooms[index].bounds));
    }
    fn fill_counterpart_adjacencies(board: &mut Board) {
        // Go room combination by room combination and check if they are touching and if they are
        // then create the doors and log the adjacency
        for first_index in 0..board.rooms.len() {
            for second_index in 0..board.rooms.len() {
                if first_index == second_index {
                    continue;
                }
                let first = &board.rooms[first_index];
                let second = &board.rooms[second_index];
                let first_bounds = first.get_bounds();
                let second_bounds = second.get_bounds();

                // If the left side of first and the right side of second are touching
                if first_bounds.left() == second_bounds.right()
                    && first_bounds.top() + 1 < second_bounds.bottom() - 1
                    && second_bounds.top() + 1 < first_bounds.bottom() - 1
                {
                    // We get the top and bottom of the overlapping section of wall
                    let top = first_bounds.top().max(second_bounds.top());
                    let bottom = first_bounds.bottom().min(second_bounds.bottom());

                    // And we get the middle of where they are touching
                    let mid = top.midpoint(bottom);

                    // Then we place the door and mark the adjacency
                    let door_pos = Vector::new(board.rooms[first_index].get_bounds().left(), mid);
                    if !(first_bounds.contains(door_pos) && second_bounds.contains(door_pos)) {
                        panic!(
                            "First:\n\
                            index: {first_index}\n\
                            bounds: {first_bounds:?}\n\
                            debug: {first:#?}\n\
                            \n\
                            Second:\n\
                            index: {second_index}\n\
                            bounds: {second_bounds:?}\n\
                            debug: {second:#?}\n\
                            \n\
                            top of connected area: {top}\n\
                            bottom of connected area: {bottom}\n\
                            midpoint of connected area: {mid}\n\
                            calculated door position: {door_pos}\n\
                            result of if first contains door pos: {}\n\
                            result of if second contains door pos: {}",
                            first_bounds.contains(door_pos),
                            second_bounds.contains(door_pos)
                        );
                    }

                    board[door_pos] = Some(Tile::Door {
                        open: false,
                        rooms: [
                            super::room::room_id(first_index as u16),
                            super::room::room_id(second_index as u16),
                        ],
                    });
                    board.rooms[first_index]
                        .add_connection(door_pos, super::room::room_id(second_index));
                    board.rooms[second_index]
                        .add_connection(door_pos, super::room::room_id(first_index));
                }
                // If the top side of first and the bottom side of second are touching
                else if first_bounds.top() == second_bounds.bottom()
                    && first_bounds.left() + 1 < second_bounds.right() - 1
                    && second_bounds.left() + 1 < first_bounds.right() - 1
                {
                    // Second verse same as the first
                    let left = first_bounds.left().max(second_bounds.left());
                    let right = first_bounds.right().min(second_bounds.right());
                    let mid = left.midpoint(right);
                    let door_pos = Vector::new(mid, first_bounds.top());
                    assert!(first_bounds.contains(door_pos));
                    assert!(second_bounds.contains(door_pos));

                    board[door_pos] = Some(Tile::Door {
                        open: false,
                        rooms: [
                            super::room::room_id(first_index as u16),
                            super::room::room_id(second_index as u16),
                        ],
                    });
                    board.rooms[first_index]
                        .add_connection(door_pos, super::room::room_id(second_index));
                    board.rooms[second_index]
                        .add_connection(door_pos, super::room::room_id(first_index));
                }

                // We don't need to account for the other 2 sides because eventually second_index
                // and first_index will be swapped
            }
        }
    }
    fn set_room_map(board: &mut Board) {
        for (id, room) in board.rooms.iter().enumerate() {
            let bounds = room.get_bounds();
            let interior_bounds = Zone::new(
                bounds.left() + 1,
                bounds.right() - 1,
                bounds.top() + 1,
                bounds.bottom() - 1,
            )
            .unwrap();
            for (position, _) in interior_bounds.scanlines() {
                board.room_map
                    [super::convert_z_order_index(position, board.axis_length).unwrap()] =
                    super::RoomIDFlagged::new(Some(super::room::room_id(id as u16)));
            }
        }
    }
}
fn validate(board: &Board) {
    for first_index in 0..board.rooms.len() {
        for second_index in 0..board.rooms.len() {
            if first_index == second_index {
                continue;
            }
            let first = &board.rooms[first_index];
            let second = &board.rooms[second_index];

            let first_bounds = first.get_bounds();
            let second_bounds = second.get_bounds();

            // Check if any rooms are subsets of others
            if first_bounds.contains(second_bounds.top_left())
                && first_bounds.contains(second_bounds.bottom_right())
            {
                panic!("Two fully overlapping rooms: {first_bounds:?} and {second_bounds:?}");
            }

            // Check that there are no out of bounds rooms
            // We only need to check overbounds because underbounds would panic
            if board.axis_length.to_inner() <= first_bounds.bottom_right().x {
                panic!(
                    "Room was out of bounds in x: {first_bounds:?} when limit is {}",
                    board.axis_length.to_inner()
                );
            }
            if board.axis_length.to_inner() <= first_bounds.bottom_right().y {
                panic!(
                    "Room was out of bounds in y: {first_bounds:?} when limit is {}",
                    board.axis_length.to_inner()
                );
            }
        }
    }
    // Count and log both the number of doors and the number of connections
    // It should be 2*connections = doors
    let mut door_count = 0;
    for tile in board.tiles.iter() {
        if let Some(super::Tile::Door { .. }) = tile {
            door_count += 1;
        }
    }
    let mut connection_count = 0;
    for room in board.rooms.iter() {
        connection_count += room.num_connections();
    }
    abes_nice_things::log!(
        "After map gen, there are {door_count} doors and {connection_count} connections between rooms"
    );

    // Make sure there are only walls on the edge of the map
    let max_index = board.axis_length.to_inner() - 1;
    for x in 0..max_index {
        if let Some(super::Tile::Wall) = board[Vector::new(x, 0)] {
        } else {
            panic!(
                "Found {:?} on edge of map at {}",
                board[Vector::new(x, 0)],
                Vector::new(x, 0)
            );
        }
        if let Some(super::Tile::Wall) = board[Vector::new(x, max_index)] {
        } else {
            panic!(
                "Found {:?} on edge of map at {}",
                board[Vector::new(x, max_index)],
                Vector::new(x, max_index)
            );
        }
    }
    for y in 0..max_index {
        if let Some(super::Tile::Wall) = board[Vector::new(0, y)] {
        } else {
            panic!(
                "Found {:?} on edge of map at {}",
                board[Vector::new(0, y)],
                Vector::new(0, y)
            );
        }
        if let Some(super::Tile::Wall) = board[Vector::new(max_index, y)] {
        } else {
            panic!(
                "Found {:?} on edge of map at {}",
                board[Vector::new(max_index, y)],
                Vector::new(max_index, y)
            );
        }
    }
}
