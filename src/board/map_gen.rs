use super::AxisLength;
use super::Board;
use super::Tile;
use super::convert_z_order_index;
use crate::Vector;
use crate::Zone;
use crate::enemy::Enemy;
use crate::math::Axis;
use crate::random::PickRandom;
use crate::random::Random;
use anyhow::Result;

// Start with a box defined by the axis length and budget.
// Recursively divide the box into two in a random axis (prefering more square) and at a random
// point along that axis in increments of 4 tiles. There is a minimum size at a room's axis can be
// in order for it to be subdivided (if either axis is too small then it does not) and at any point
// past the 4th round of divisions, subdivisions can stop with the chance increasing as the room
// gets smaller. Each subdivision gets a proportional amount of its parent's budget based on how
// much of the parent's space it got.
//
// adjacent rooms get doors connecting them at the midpoint in the shared
// section of wall

// Because all of this is happening in a different thread, we do not need to care about
// performance*

pub fn generate(
    axis_length: AxisLength,
    desired_viewport: Vector<usize>,
    budget: usize,
) -> Result<Board> {
    let mut rooms = Vec::new();
    rooms.push(Room {
        bounds: Zone::from_vectors(
            Vector::ZERO,
            Vector::new(axis_length.to_inner() - 1, axis_length.to_inner() - 1),
        ),
        children: None,
        budget,
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
    let spawn_budget = Room::remove_budget_of_spawn(&mut rooms, 0);
    Room::reallocate_spawn_budget(&mut rooms, 0, spawn_budget);
    Room::place_enemies(&mut board, &rooms, 0);
    //validate(&board);
    Ok(board)
}
struct Room {
    /// These bounds include the walls meaning that there will be overlapping edges of adjacent
    /// rooms
    bounds: Zone<usize>,
    /// If this rooms has children, then it is the indices of those children
    children: Option<[usize; 2]>,
    /// The budget for enemies in this room
    budget: usize,
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
        if rooms[index]
            .bounds
            .height()
            .max(rooms[index].bounds.width())
            <= Room::MINIMUM_AXIS
        {
            return;
        }

        // Random chance of stopping division based on size
        if depth > Room::MINIMUM_STOP_DEPTH
            && (smallest_axis_length as f64) < (crate::random::random() - 1.0) * max_early_stop
        {
            return;
        }

        // We are going to subdivide

        // Deciding the axis for division
        // 1 in 4 chance of dividing the smallest axis instead of the bigger one
        let division_axis =
            if (u8::random() & 0b11) == 0 && smallest_axis_length > Room::MINIMUM_AXIS {
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
        let relative_split_point = ((((crate::random::random() + crate::random::random()) / 2.0)
            - 1.0)
            * (range_end - range_start - 8) as f64) as usize
            + 4;
        let split_point = relative_split_point + range_start;
        let total_budget = rooms[index].budget;
        let budget_split_point = (relative_split_point as f64 / (range_end - range_start) as f64
            * total_budget as f64) as usize;

        // Creating children

        // Top left child
        rooms.push(Room {
            // Big split point, big bounds
            bounds: Zone::from_vectors(
                rooms[index].bounds.top_left(),
                match division_axis {
                    Axis::Horizontal => Vector::new(split_point, rooms[index].bounds.bottom()),
                    Axis::Vertical => Vector::new(rooms[index].bounds.right(), split_point),
                },
            ),
            children: None,
            budget: budget_split_point,
        });
        // Bottom right child
        rooms.push(Room {
            // Big split point, small bounds
            bounds: Zone::from_vectors(
                match division_axis {
                    Axis::Horizontal => Vector::new(split_point, rooms[index].bounds.top()),
                    Axis::Vertical => Vector::new(rooms[index].bounds.left(), split_point),
                },
                rooms[index].bounds.bottom_right(),
            ),
            children: None,
            budget: total_budget - budget_split_point,
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
    /// Returns the budget to be reallocated
    fn remove_budget_of_spawn(rooms: &mut Vec<Room>, index: usize) -> usize {
        if let Some(children) = rooms[index].children {
            // because of how it subdivides, the first child will always be towards the top left
            // aka spawn
            return Room::remove_budget_of_spawn(rooms, children[0]);
        }

        // We are at the spawn room
        std::mem::replace(&mut rooms[index].budget, 0)
    }
    /// Reallocate the budget what was going to the spawn room to a random room in the bottom right
    /// quadrant
    fn reallocate_spawn_budget(rooms: &mut Vec<Room>, index: usize, budget: usize) {
        if let Some(children) = rooms[index].children {
            // Make it so that no matter what it does not go back in the spawn room
            let recurse_child = if index == 0 {
                1
            } else {
                (u8::random() & 0b1) as usize
            };
            Room::reallocate_spawn_budget(rooms, children[recurse_child], budget);
            return;
        }

        // We have hit the lotto winner
        rooms[index].budget += budget;
    }
    fn place_enemies(board: &mut Board, rooms: &Vec<Room>, index: usize) {
        // 7,931,287th verse, same as the first
        if let Some(children) = rooms[index].children {
            Room::place_enemies(board, rooms, children[0]);
            Room::place_enemies(board, rooms, children[1]);
            return;
        }
        let mut budget = rooms[index].budget;
        // Have to account for the walls
        let spawn_bounds = rooms[index].bounds.shrink_by(1).unwrap();

        // First we spawn few high tier centers then spawn lower tier enemies around them
        let num_centers = (spawn_bounds.area() / 500) + 1;
        let mut centers = Vec::with_capacity(num_centers);
        for _ in 0..num_centers {
            // Find an empty space in the room
            // We will attempt 10 times per center
            for _ in 0..10 {
                let position = spawn_bounds.generate();
                if board.is_enemy_at_position(position) {
                    continue;
                }
                if let Some(vtable) = Enemy::pick_vtable_from_budget(&mut budget, None) {
                    centers.push(board.add_enemy(Enemy::new(vtable, position)));
                } else {
                    // If it was unable to place the any enemy due to budget then there is no point
                    // in continuing
                    return;
                }
                break;
            }
        }

        // Now that we have our centers we need to go until we run out of budget and place things
        // near them in a round robin in a 10 radius square clamped to the edge of the room
        while budget > 0 {
            for center in centers.iter() {
                let center_pos = board[*center].as_ref().unwrap().get_position();
                // We don't need to worry about overflowing out of the board because it will be
                // handled by the subset
                let spawn_bounds = Zone::new(
                    center_pos.x.saturating_sub(10),
                    center_pos.x + 10,
                    center_pos.y.saturating_sub(10),
                    center_pos.y + 10,
                )
                .unwrap()
                .subset(&rooms[index].bounds)
                .unwrap();
                let max_tier = board[*center].as_ref().unwrap().get_vtable().tier;
                // usual 10 attempts max
                for _ in 0..10 {
                    let position = spawn_bounds.generate();
                    if board.is_enemy_at_position(position) {
                        continue;
                    }
                    if let Some(vtable) =
                        Enemy::pick_vtable_from_budget(&mut budget, Some(max_tier))
                    {
                        board.add_enemy(Enemy::new(vtable, position));
                        break;
                    }
                    // If we reach this then we ran out of budget so there is no point in
                    // continuing
                    return;
                }
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
        // Ensure each connection has a door
        for (position, _) in board.rooms[first_index].connections.iter() {
            if let Some(super::Tile::Door { .. }) = board[*position] {
            } else {
                panic!("There was a connection without a door at {position}");
            }
        }
        // Ensure each connection is mutual
        for (position, connectee) in board.rooms[first_index].connections.iter() {
            assert!(
                board[*connectee]
                    .connections
                    .contains(&(*position, super::room::room_id(first_index)))
            );
        }
    }
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
    // Ensure there are no overlapping enemies and all enemies are on empty tiles
    for (first_index, first_enemy) in board.enemies.iter().enumerate() {
        let first_enemy = first_enemy.as_ref().unwrap();
        assert!(board[first_enemy.get_position()].is_none());
        for (second_index, second_enemy) in board.enemies.iter().enumerate() {
            if first_index == second_index {
                continue;
            }
            let second_enemy = second_enemy.as_ref().unwrap();
            assert_ne!(first_enemy.get_position(), second_enemy.get_position())
        }
    }
}
