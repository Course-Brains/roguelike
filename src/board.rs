mod axis_length;
pub mod tile;
pub use axis_length::AxisLength;
pub mod map_gen;
mod room;
use crate::random::Random;
use room::Room;
use room::RoomID;
use room::RoomIDFlagged;

use crate::Vector;
use crate::Zone;
use crate::enemy::Enemy;
use crate::math::Direction;
use crate::state::State;
use abes_nice_things::Number;
use abes_nice_things::PrimAs;
use anyhow::{Context, Result, bail};
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;
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
    /// This is used to get which room the interior coordinate is a part of. This does not include
    /// walls or doors. This also has the same restrictions as tiles
    room_map: Vec<RoomIDFlagged>,
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
    rooms: Vec<Room>,
}
// Helpers
impl Board {
    /// Creates a blank board which is not populated by tile objects or map objects and is
    /// therefore not valid
    pub fn new(axis_length: AxisLength, desired_viewport: Vector<usize>) -> Result<Board> {
        Ok(Board {
            tiles: Board::create_blank_tile_array(axis_length)?,
            room_map: vec![
                RoomIDFlagged::new(None);
                axis_length.to_inner() * axis_length.to_inner()
            ],
            axis_length,
            viewport_size: desired_viewport
                .min(Vector::new(axis_length.to_inner(), axis_length.to_inner())),
            enemies: Vec::new(),
            local_turns: 0,
            rooms: Vec::new(),
        })
    }
    pub fn axis_length(&self) -> AxisLength {
        self.axis_length
    }
    fn add_room(&mut self, room: Room) -> RoomID {
        self.rooms.push(room);
        room::room_id(self.rooms.len() - 1)
    }
    /// First we run the thinkers
    ///
    /// Then we pathfind
    pub fn increment(state: &mut State) {
        state.board.local_turns += 1;
        Board::run_thinkers(state);
        Board::pathfind(state);
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
    /// Zeros the cursor and draws the tiles onto the screen and clears the screen, this is the first layer of rendering.
    ///
    /// Additionally it draws the border of the viewport
    pub fn render_tiles(&self, viewport: Zone<usize>) {
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
    pub fn render_enemies(&self, viewport: Zone<usize>) {
        // The weird iterator stuff ensures that we only are rendering enemies which are alive and
        // on screen on top of getting us the on screen position of that enemy
        for (position, (character, style)) in self
            .enemies
            .iter()
            .filter_map(|enemy| enemy.as_ref())
            .filter(|enemy| viewport.contains(enemy.position))
            .map(|enemy| (enemy.position - viewport.top_left(), enemy.render()))
        {
            print!(
                "\x1b[{};{}H{style}{character}\x1b[0m",
                position.y + 1,
                position.x + 1
            );
        }
    }
}

// TILES
impl Board {
    /// ALWAYS ensure this matches the implementations for indexing into the tiles.
    /// This is the maximum length of each axis for the board.
    ///
    /// This is an exclusive bounds when referring to indices
    const MAX_AXIS_LENGTH: usize = 0b1 << Board::MAX_AXIS_BITS; // 1024
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
        if position.x >= T::prim_from(self.axis_length.to_inner())
            || position.y >= T::prim_from(self.axis_length.to_inner())
        {
            return None;
        }
        Some(&self[position.prim_as()])
    }
    /// Checks if moving from a known to be valid position in a given direction will still be on
    /// the board
    pub fn is_move_on_board(&self, start: Vector<usize>, direction: Direction) -> bool {
        match direction {
            Direction::Up => start.y > 0,
            Direction::Down => start.y < (self.axis_length.to_inner() - 1),
            Direction::Left => start.x > 0,
            Direction::Right => start.x < (self.axis_length.to_inner() - 1),
        }
    }
    /// Checks if the player can move from a known valid position in a given direction
    pub fn player_can_move(&self, start: Vector<usize>, direction: Direction) -> bool {
        self.is_move_on_board(start, direction)
            && self[start + direction].is_none_or(|tile| !tile.is_player_collidable())
            && !self.is_enemy_at_position(start + direction)
    }
    pub fn get_room_id_of_coord(&self, position: Vector<usize>) -> Option<RoomID> {
        self.room_map[convert_z_order_index(position, self.axis_length).unwrap()].get_id()
    }
    pub fn get_possible_room_ids_at_position(&self, position: Vector<usize>) -> Vec<RoomID> {
        if let Some(room) = self.get_room_id_of_coord(position) {
            vec![room]
        } else if let Some(Tile::Door { rooms, .. }) = self[position] {
            rooms.to_vec()
        } else {
            Vec::new()
        }
    }
}

// ENEMIES
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyID(pub usize);
impl Board {
    pub fn add_enemy(&mut self, enemy: crate::enemy::Enemy) -> EnemyID {
        self.enemies.push(Some(enemy));
        return EnemyID(self.enemies.len() - 1);
    }
    /// This requires immutable access to that specific enemy
    pub fn get_enemy(&self, id: EnemyID) -> &Option<Enemy> {
        &self.enemies[id.0]
    }
    /// This requires mutable access to that specific enemy
    pub fn get_enemy_mut(&mut self, id: EnemyID) -> &mut Option<Enemy> {
        &mut self.enemies[id.0]
    }
    /// This requires mutable accesss to all enemies
    pub fn run_thinkers(state: &mut State) {
        for index in 0..state.board.enemies.len() {
            if state.board.enemies[index].is_some() {
                let vtable = state.board.enemies[index].as_ref().unwrap().get_vtable();
                (vtable.think)(state, EnemyID(index));
            }
        }
    }
    /// This requires mutable access to all enemies
    pub fn pathfind(state: &mut State) {
        Board::inter_room_pathfind(state);
        Board::intra_room_pathfind(state);
    }
    fn intra_room_pathfind(state: &mut State) {
        for index in 0..state.board.enemies.len() {
            if state.board.enemies[index].is_some() {
                Enemy::intra_room_pathfind(state, EnemyID(index));
            }
        }
    }
    /// Inter room pathfinding implemented as A* considering only the rooms
    fn inter_room_pathfind(state: &mut State) {
        for id in 0..state.board.enemies.len() {
            // Figuring out if we need to do anything
            // If there is no enemy then we can't pathfind
            if state.board.enemies[id].is_none() {
                continue;
            }

            // If we are still walking then keep walking
            if let Some(walk_time) = state.board.enemies[id].as_ref().unwrap().walk_time {
                state.board.enemies[id].as_mut().unwrap().walk_time =
                    std::num::NonZeroUsize::new(walk_time.get() - 1);
                continue;
            }
            let enemy = state.board.enemies[id].as_ref().unwrap();
            // If the enemy doesn't want to go anywhere or already knows where to go or is asleep
            // then we don't need to do anything
            if enemy.end_goal.is_none() || !enemy.flags.is_awake() {
                continue;
            }

            let possible_end_goal_rooms = state
                .board
                .get_possible_room_ids_at_position(enemy.end_goal.unwrap());
            let possible_start_rooms = state
                .board
                .get_possible_room_ids_at_position(enemy.position);
            // Enemies MUST always be either within a room or on a door
            assert!(!possible_end_goal_rooms.is_empty());
            assert!(!possible_start_rooms.is_empty());
            // If it is already in the room it needs to be in then we don't have to do anything
            if possible_start_rooms
                .iter()
                .any(|start| possible_end_goal_rooms.contains(start))
            {
                state.board.enemies[id].as_mut().unwrap().move_target = enemy.end_goal;
                continue;
            }

            // Sadly we have to actually do our job, ew
            struct Heuristic {
                /// The estimate at the remaining travel cost from this position
                remaining_heuristic: usize,
                /// The known travel cost to this position
                known_cost: usize,
                /// The position
                position: Vector<usize>,
                /// The room it is entering
                room: RoomID,
                /// The room which was the previous room in the path taken
                backpath: Option<RoomID>,
            }
            impl Heuristic {
                fn new(
                    position: Vector<usize>,
                    goal: Vector<usize>,
                    known_cost: usize,
                    room: RoomID,
                    backpath: Option<RoomID>,
                ) -> Self {
                    Heuristic {
                        remaining_heuristic: position.abs_diff(goal).sum_axes(),
                        known_cost,
                        position,
                        room,
                        backpath,
                    }
                }
            }
            impl PartialEq for Heuristic {
                fn eq(&self, other: &Self) -> bool {
                    self.remaining_heuristic + self.known_cost
                        == other.remaining_heuristic + other.known_cost
                }
            }
            impl Eq for Heuristic {}
            impl PartialOrd for Heuristic {
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    // Yes this ordering is intentional
                    (other.remaining_heuristic + other.known_cost)
                        .partial_cmp(&(self.remaining_heuristic + self.known_cost))
                }
            }
            impl Ord for Heuristic {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    self.partial_cmp(other).unwrap()
                }
            }
            // Setup
            let mut visited = HashSet::new();
            let mut to_visit = BinaryHeap::new();
            let mut backpath = HashMap::new();
            // Because we have ensured that inter room pathfinding must be done, the last room will
            // never be the same as the start room and the only way for this to be the same as the
            // start room is for the start and target to be in the same room, we know this will not
            // be None when we are done traversing the rooms
            let mut last_room = None;
            for start_room in possible_start_rooms.iter() {
                to_visit.push(Heuristic::new(
                    enemy.position,
                    enemy.end_goal.unwrap(),
                    0,
                    *start_room,
                    None,
                ));
            }

            // Traversing the rooms
            while let Some(current) = to_visit.pop() {
                if !visited.insert(current.room) {
                    continue;
                }
                if let Some(backpath_id) = current.backpath {
                    backpath.insert(current.room, backpath_id);
                }
                last_room = Some(current.room);

                let room = &state.board[current.room];
                for (position, connectee) in room.connections.iter() {
                    if visited.contains(connectee) {
                        continue;
                    }
                    // If the door is closed then it can't walk through it
                    if let Some(Tile::Door { open: true, .. }) = state.board[*position] {
                    } else {
                        continue;
                    }
                    // If the doors share a wall then we have to add two because it has to walk
                    // into the room then back out instead of travelling through the wall
                    let additional =
                        if current.position.x == position.x || current.position.y == position.y {
                            2
                        } else {
                            0
                        };

                    to_visit.push(Heuristic::new(
                        *position,
                        enemy.end_goal.unwrap(),
                        current.known_cost
                            + current.position.abs_diff(*position).sum_axes()
                            + additional,
                        *connectee,
                        Some(current.room),
                    ));
                }
            }
            // See above
            assert!(last_room.is_some());

            // Following the path back
            // If this ever breaks early due to the loop condition failing then pathfinding has
            // failed and it won't move even though it is trying to
            while let Some(next) = backpath.get(&last_room.unwrap()) {
                // We have found the room to go to
                if possible_start_rooms.contains(next) {
                    // We don't need to worry about if the condition in this loop won't be met
                    // because board validation ensures each connection is mutual
                    for (position, connection) in state.board[*next].connections.clone().into_iter()
                    {
                        // We have found the specific door to walk to
                        if possible_start_rooms.contains(&connection) {
                            let enemy = state.board.enemies[id].as_mut().unwrap();
                            enemy.move_target = Some(position);
                            enemy.walk_time =
                                std::num::NonZeroUsize::new((u8::random() & 0b11) as usize + 4);
                            break;
                        }
                    }
                    break;
                }
            }
        }
    }
    //TODO: Once the player is added, make this check for the player's position
    pub fn enemy_can_move(&self, start: Vector<usize>, direction: Direction) -> bool {
        // Is the target location on the board?
        if !self.is_move_on_board(start, direction) {
            return false;
        }
        let new_pos = start + direction;
        // Is there a blocking tile?
        if self[new_pos].is_some_and(|tile| tile.is_enemy_collidable()) {
            return false;
        }
        // Is there an enemy there?
        if self.is_enemy_at_position(new_pos) {
            return false;
        }
        true
    }
    /// This requires immutable access to all enemies
    pub fn is_enemy_at_position(&self, position: Vector<usize>) -> bool {
        self.enemies.iter().any(|enemy| {
            enemy
                .as_ref()
                .is_some_and(|enemy| enemy.position == position)
        })
    }
    pub fn count_enemies(&self) -> usize {
        self.enemies.len()
    }
    pub fn get_enemy_at_position(&self, position: Vector<usize>) -> Option<EnemyID> {
        for (id, enemy) in self.enemies.iter().enumerate() {
            if let Some(enemy) = enemy
                && enemy.position == position
            {
                return Some(EnemyID(id));
            }
        }
        None
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
impl std::ops::Index<EnemyID> for Board {
    type Output = Option<Enemy>;
    fn index(&self, index: EnemyID) -> &Self::Output {
        self.get_enemy(index)
    }
}
impl std::ops::IndexMut<EnemyID> for Board {
    fn index_mut(&mut self, index: EnemyID) -> &mut Self::Output {
        self.get_enemy_mut(index)
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
    debug_assert!(axis_length.to_inner() <= Board::MAX_AXIS_LENGTH);

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
impl std::ops::Index<RoomID> for Board {
    type Output = Room;
    fn index(&self, index: RoomID) -> &Self::Output {
        &self.rooms[index.get_inner() as usize]
    }
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
