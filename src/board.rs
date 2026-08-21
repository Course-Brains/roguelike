mod axis_length;
pub mod tile;
pub use axis_length::AxisLength;
pub mod map_gen;
mod room;
use room::Room;
use room::RoomID;
use room::RoomIDFlagged;
mod projectile;

use crate::Vector;
use crate::Zone;
use crate::enemy::Enemy;
use crate::math::Direction;
use crate::state::State;
use abes_nice_things::MaxVec;
use abes_nice_things::Number;
use abes_nice_things::PrimAs;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::{Context, Result, bail};
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;
use tile::Tile;
mod walk_trigger;
use crate::spell::Spell;
use abes_nice_things::log;
pub use projectile::Projectile;
pub use projectile::ProjectileType;
pub use walk_trigger::WalkTrigger;

/// This contains all data which is tied to the specific map, which is everything that does not
/// carry over between maps.
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
    /// The size of the viewport, the center will tend towards the top left.
    ///
    /// This will be recalculated on load
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
    map_type: MapType,
    /// The bosses and their last known valid position (empty positions so we don't overwrite
    /// something and break things)
    bosses: Vec<(EnemyID, Vector<usize>)>,
    /// Projectiles in the air, you cannot assume the index of a projectile will be consistent
    projectiles: Vec<Projectile>,
    /// Spell circles that are not within the bounds of a room (currently only case where this
    /// would happen is an open door)
    floating_spell_circles: Vec<(Vector<usize>, Spell)>,
    /// Visual graphics which do not affect logic which must only exist for a short time (less than
    /// one turn)
    short_render_specials: Vec<RenderSpecial>,
}
impl ToBinary for Board {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        self.axis_length.to_binary(binary)?;
        self.tiles.len().to_binary(binary)?;
        for tile in self.tiles.iter() {
            tile.as_ref().to_binary(binary)?;
        }
        self.room_map.to_binary(binary)?;
        // viewport size does not get saved
        self.enemies.len().to_binary(binary)?;
        for enemy in self.enemies.iter() {
            enemy.as_ref().to_binary(binary)?;
        }
        self.local_turns.to_binary(binary)?;
        self.rooms.to_binary(binary)?;
        self.map_type.to_binary(binary)?;
        self.bosses.len().to_binary(binary)?;
        for (boss, last_good) in self.bosses.iter() {
            boss.to_binary(binary)?;
            last_good.to_binary(binary)?;
        }
        self.projectiles.to_binary(binary)?;
        self.floating_spell_circles.len().to_binary(binary)?;
        for (position, spell) in self.floating_spell_circles.iter() {
            position.to_binary(binary)?;
            spell.to_binary(binary)?;
        }
        // Short render specials do not live long enough to reach a save
        Ok(())
    }
}
impl FromBinary for Board {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        let axis_length = AxisLength::from_binary(binary)?;
        Ok(Board {
            tiles: <Vec<Option<Tile>>>::from_binary(binary)?,
            room_map: <Vec<RoomIDFlagged>>::from_binary(binary)?,
            axis_length,
            // Viewport has to be recalculated on load for reasons that should be obvious
            viewport_size: crate::calc_desired_dimensions(crate::get_terminal_size())
                .min(Vector::equal_from_axis(axis_length.to_inner())),
            enemies: <Vec<Option<Enemy>>>::from_binary(binary)?,
            local_turns: usize::from_binary(binary)?,
            rooms: <Vec<Room>>::from_binary(binary)?,
            map_type: MapType::from_binary(binary)?,
            bosses: <Vec<(EnemyID, Vector<usize>)>>::from_binary(binary)?,
            projectiles: <Vec<Projectile>>::from_binary(binary)?,
            floating_spell_circles: <Vec<(Vector<usize>, Spell)>>::from_binary(binary)?,
            // Short render specials do not live long enough to reach a save
            short_render_specials: Vec::new(),
        })
    }
}

// Helpers
impl Board {
    /// Creates a blank board which is not populated by tile objects or map objects and is
    /// therefore not valid
    pub fn new(
        axis_length: AxisLength,
        desired_viewport: Vector<usize>,
        map_type: MapType,
    ) -> Result<Board> {
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
            map_type,
            bosses: Vec::new(),
            projectiles: Vec::new(),
            floating_spell_circles: Vec::new(),
            short_render_specials: Vec::new(),
        })
    }
    pub fn recalc_viewport(&mut self, desired: Vector<usize>) {
        self.viewport_size = desired.min(Vector::equal_from_axis(self.axis_length().to_inner()));
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
        Board::decriment_enemy_effects(state);
        Board::pathfind(state);
        Board::run_thinkers(state);
        Board::update_projectiles(state);
        state.board.update_boss_last_known_positions();
    }
    /// Update the last known good positions of bosses and spawn exits where needed
    fn update_boss_last_known_positions(&mut self) {
        for index in 0..self.bosses.len() {
            // Dead bosses don't update positions but do need an exit
            // Yes this does spawn the exit every tick, it doesn't matter much
            if self[self.bosses[index].0].is_none() {
                let pos = self.bosses[index].1;
                self[pos] = Some(Tile::WalkTrigger(WalkTrigger::Exit));
                continue;
            }
            let enemy_pos = self[self.bosses[index].0].as_ref().unwrap().get_position();
            if self[enemy_pos].is_none() {
                self.bosses[index].1 = enemy_pos
            }
        }
    }
    /// Assumes a valid enemy id, a live enemy and a valid exit spawn at enemy position
    pub fn register_boss(&mut self, id: EnemyID) {
        self.bosses
            .push((id, self[id].as_ref().unwrap().get_position()))
    }
    pub fn log_boss_tracking(&self) {
        log!("Boss tracking: {:?}", self.bosses)
    }
    fn update_projectiles(state: &mut State) {
        let viewport = state.calculate_viewport();
        let mut index = 0;
        while index < state.board.projectiles.len() {
            if Projectile::step(state, index, &viewport) {
                state.board.projectiles.swap_remove(index);
            } else {
                index += 1;
            }
        }
    }
    pub fn get_viewport_size(&self) -> Vector<usize> {
        self.viewport_size
    }
    pub fn get_local_turn(&self) -> usize {
        self.local_turns
    }
    #[cold]
    pub fn open_all_doors(&mut self) {
        for tile in self.tiles.iter_mut() {
            if let Some(Tile::Door { open, .. }) = tile {
                *open = true;
            }
        }
    }
    #[cold]
    pub fn wake_all_enemies(&mut self) {
        for enemy in self.enemies.iter_mut() {
            if let Some(enemy) = enemy {
                enemy.flags.wake()
            }
        }
    }
    /// Creates a valid but empty shop without anything to buy and no exit
    pub fn create_blank_shop(desired_viewport: Vector<usize>) -> Board {
        let mut out = Board::new(AxisLength::Tiny, desired_viewport, MapType::Shop).unwrap();
        let edge = AxisLength::Tiny.to_inner() - 1;

        for i in 0..AxisLength::Tiny.to_inner() {
            for pos in [(i, 0), (0, i), (i, edge), (edge, i)]
                .map(|(x, y)| Vector::new(x, y))
                .into_iter()
            {
                out[pos] = Some(Tile::Wall);
            }
        }

        out
    }
    pub fn get_spell_circle_at_position(&self, position: Vector<usize>) -> Option<&Spell> {
        for (circle_position, spell) in match self.get_room_id_of_coord(position) {
            Some(room) => &self[room].spell_circles,
            None => &self.floating_spell_circles,
        }
        .iter()
        {
            if *circle_position == position {
                return Some(spell);
            }
        }
        None
    }
    pub fn add_projectile(&mut self, projectile: Projectile) {
        self.projectiles.push(projectile);
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
    pub fn render_tiles(state: &State, viewport: Zone<usize>, buffer: &mut impl Write) {
        // Putting the cursor in the top corner
        write!(buffer, "\x1b[H").unwrap();
        for (position, last) in viewport.scanlines() {
            if let Some(tile) = state.board[position] {
                let (ch, style) = tile.render(state, position);
                match style {
                    Some(style) => write!(buffer, "{style}{ch}\x1b[0m").unwrap(),
                    None => write!(buffer, "{ch}").unwrap(),
                }
            } else {
                write!(buffer, " ").unwrap();
            }
            if last {
                // erase until end of line and draw right border
                writeln!(buffer, "{}\x1b[0K", Board::VIEWPORT_BORDER_RIGHT).unwrap();
            }
        }
        // erase from cursor to end of screen and draw bottom of border
        write!(
            buffer,
            "{}{}\x1b[0J",
            Board::VIEWPORT_BORDER_BOTTOM
                .to_string()
                .repeat(viewport.width()),
            Board::VIEWPORT_BORDER_CORNER
        )
        .unwrap();
    }
    /// Moves the cursor about to draw the enemies, this is the second layer of rendering.
    pub fn render_enemies(state: &mut State, viewport: Zone<usize>, buffer: &mut impl Write) {
        for index in 0..state.board.enemies.len() {
            if state.board.enemies[index].is_none() {
                continue;
            }
            let enemy = state.board.enemies[index].as_ref().unwrap();
            if !viewport.contains(enemy.get_position()) {
                continue;
            }
            let screen_position = enemy.get_position() - viewport.top_left() + 1;
            let (ch, style) = Enemy::render(state, EnemyID(index));
            write!(
                buffer,
                "\x1b[{};{}H{style}{ch}\x1b[0m",
                screen_position.y, screen_position.x
            )
            .unwrap();
        }
    }
    pub fn render_projectiles(&self, viewport: Zone<usize>, buffer: &mut impl Write) {
        for projectile in self.projectiles.iter() {
            if !viewport.contains(projectile.position()) {
                continue;
            }
            let screen_position = projectile.position() - viewport.top_left() + 1;
            let (ch, style) = projectile.r#type().render();
            write!(buffer, "\x1b[{};{}H", screen_position.y, screen_position.x).unwrap();
            if let Some(style) = style {
                write!(buffer, "{style}").unwrap();
            }
            write!(buffer, "{ch}").unwrap();
            if style.is_some() {
                write!(buffer, "\x1b[0m").unwrap();
            }
        }
    }
    pub fn render_short_specials(&self, viewport: Zone<usize>, buffer: &mut impl Write) {
        for special in self.short_render_specials.iter() {
            if !viewport.contains(special.position) {
                continue;
            }
            let screen_position = special.position - viewport.top_left() + 1;
            if let Some(style) = special.style {
                write!(
                    buffer,
                    "\x1b[{};{}H{style}{}\x1b[0m",
                    screen_position.y, screen_position.x, special.ch
                )
                .unwrap();
            } else {
                write!(
                    buffer,
                    "\x1b[{};{}H{}",
                    screen_position.y, screen_position.x, special.ch
                )
                .unwrap()
            }
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
    pub fn get_possible_room_ids_at_position(&self, position: Vector<usize>) -> MaxVec<RoomID, 3> {
        if let Some(room) = self.get_room_id_of_coord(position) {
            MaxVec::from_array([room]).unwrap()
        } else if let Some(Tile::Door { rooms, .. }) = self[position] {
            MaxVec::from_array(rooms).unwrap()
        } else {
            MaxVec::new()
        }
    }
    /// Chances are you don't need a mutable reference, be careful
    pub fn get_room_mut(&mut self, room: RoomID) -> &mut Room {
        &mut self.rooms[room.get_inner() as usize]
    }
}

// ENEMIES
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EnemyID(pub usize);
impl ToBinary for EnemyID {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        self.0.to_binary(binary)
    }
}
impl FromBinary for EnemyID {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(EnemyID(usize::from_binary(binary)?))
    }
}
impl Board {
    pub fn add_enemy(&mut self, enemy: crate::enemy::Enemy) -> EnemyID {
        self.enemies.push(Some(enemy));
        Enemy::inital_room_memoize(self, EnemyID(self.enemies.len() - 1));
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
            if let Some(enemy) = state.board.enemies[index].as_ref()
                && enemy.flags.should_path()
            {
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

            let enemy = state.board.enemies[id].as_ref().unwrap();
            let should_log = enemy.flags.should_inter_pathfind_log();
            // If the enemy doesn't want to go anywhere or already knows where to go or is asleep
            // then we don't need to do anything
            if enemy.end_goal.is_none() || !enemy.flags.is_awake() {
                if should_log {
                    state.board.enemies[id].as_mut().unwrap().log(
                        "Decided not to \
                    inter room pathfind because it is either asleep or has no goal"
                            .to_string(),
                    );
                }
                continue;
            }

            let possible_end_goal_rooms = state
                .board
                .get_possible_room_ids_at_position(enemy.end_goal.unwrap());
            let possible_start_rooms = state
                .board
                .get_possible_room_ids_at_position(enemy.get_position());
            let end_goal = enemy.end_goal;
            if should_log {
                let position = enemy.get_position();
                state.board[EnemyID(id)].as_mut().unwrap().log(format!(
                    "Starting pathfind with end goal {} (rooms {:?}) and \
                        current position {} (rooms {:?})",
                    end_goal.unwrap(),
                    possible_end_goal_rooms.as_slice(),
                    position,
                    possible_start_rooms.as_slice()
                ));
            }
            // Enemies MUST always be either within a room or on a door
            assert!(!possible_end_goal_rooms.is_empty());
            assert!(!possible_start_rooms.is_empty());
            // If it is already in the room it needs to be in then we don't have to do anything
            if possible_start_rooms
                .iter()
                .any(|start| possible_end_goal_rooms.contains(start))
            {
                if should_log {
                    state.board[EnemyID(id)].as_mut().unwrap().log(format!(
                        "Already in target room \
                            so stopping"
                    ));
                }
                state.board.enemies[id].as_mut().unwrap().move_target = end_goal;
                continue;
            }

            // Sadly we have to actually do our job, ew
            #[derive(Debug)]
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
                    same_wall: bool,
                ) -> Self {
                    let mut heuristic = position.abs_diff(goal).sum_axes();
                    if same_wall {
                        heuristic += 2;
                    }
                    Heuristic {
                        remaining_heuristic: heuristic,
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
            let enemy = state.board[EnemyID(id)].as_ref().unwrap();
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
                    enemy.get_position(),
                    enemy.end_goal.unwrap(),
                    0,
                    *start_room,
                    None,
                    false,
                ));
            }

            // Traversing the rooms
            while let Some(current) = to_visit.pop() {
                if !visited.insert(current.room) {
                    continue;
                }
                if should_log {
                    state.board[EnemyID(id)].as_mut().unwrap().log(format!(
                        "Starting visit of room with heuristic: {current:?}"
                    ));
                }
                if let Some(backpath_id) = current.backpath {
                    backpath.insert(current.room, backpath_id);
                }
                last_room = Some(current.room);
                // We have found our path
                if possible_end_goal_rooms.contains(&current.room) {
                    break;
                }

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
                        end_goal.unwrap(),
                        current.known_cost
                            + current.position.abs_diff(*position).sum_axes()
                            + additional,
                        *connectee,
                        Some(current.room),
                        position.x == current.position.x || position.y == current.position.y,
                    ));
                }
            }
            // See above
            assert!(last_room.is_some());
            let enemy = state.board[EnemyID(id)].as_mut().unwrap();

            if should_log {
                enemy.log("Starting path retrace".to_string());
            }

            // Following the path back
            // If this ever breaks early due to the loop condition failing then pathfinding has
            // failed and it won't move even though it is trying to
            let mut next = last_room.unwrap();
            loop {
                if should_log {
                    state.board[EnemyID(id)]
                        .as_mut()
                        .unwrap()
                        .log(format!("   Checking room {next:?}"));
                }
                // This shenaniganily goes through the connections to other rooms and finds out if
                // there is a connection which goes to a start room and is open and if there is
                // then it sets the walk target
                if let Some(position) = state.board[next]
                    .connections
                    .iter()
                    .filter(|(position, connection)| {
                        possible_start_rooms.contains(connection)
                            && matches!(state.board[position], Some(Tile::Door { open: true, .. }))
                            && *position
                                != state.board[EnemyID(id)].as_ref().unwrap().get_position()
                    })
                    .map(|(position, _)| *position)
                    .next()
                {
                    let enemy = state.board[EnemyID(id)].as_mut().unwrap();
                    if should_log {
                        enemy.log(format!("    Decided to walk to door at {position}"));
                    }
                    enemy.move_target = Some(position);
                    break;
                }
                if let Some(backpath) = backpath.get(&next) {
                    next = *backpath;
                } else {
                    break;
                }
            }
        }
    }
    pub fn enemy_can_move(state: &State, start: Vector<usize>, direction: Direction) -> bool {
        // Is the target location on the board?
        if !state.board.is_move_on_board(start, direction) {
            return false;
        }
        let new_pos = start + direction;
        // Is the player there
        if new_pos == state.player.position {
            return false;
        }
        // Is there a blocking tile?
        if state.board[new_pos].is_some_and(|tile| tile.is_enemy_collidable()) {
            return false;
        }
        // Is there an enemy there?
        if state.board.is_enemy_at_position(new_pos) {
            return false;
        }
        true
    }
    /// This requires immutable access to all enemies
    pub fn is_enemy_at_position(&self, position: Vector<usize>) -> bool {
        let possible_rooms = self.get_possible_room_ids_at_position(position);
        if possible_rooms.is_empty() {
            return false;
        }
        self[possible_rooms[0]].enemies.iter().any(|id| {
            self[*id]
                .as_ref()
                .is_some_and(|enemy| enemy.get_position() == position)
        })
    }
    pub fn get_enemy_at_position(&self, position: Vector<usize>) -> Option<EnemyID> {
        let possible_rooms = self.get_possible_room_ids_at_position(position);
        if possible_rooms.is_empty() {
            return None;
        }
        for id in self[possible_rooms[0]].enemies.iter() {
            if self[*id]
                .as_ref()
                .is_some_and(|enemy| enemy.get_position() == position)
            {
                return Some(*id);
            }
        }
        None
    }
    pub fn decriment_enemy_effects(state: &mut State) {
        for id in 0..state.board.enemies.len() {
            if state.board.enemies[id].is_some() {
                let finished = state.board.enemies[id]
                    .as_mut()
                    .unwrap()
                    .effects
                    .decriment();
                crate::effect::EffectTracker::run_on_ends(
                    state,
                    crate::state::Entity::Enemy(EnemyID(id)),
                    finished,
                );
            }
        }
    }
    pub fn reset_took_damage_flags(&mut self) {
        for enemy in self.enemies.iter_mut().filter_map(|enemy| enemy.as_mut()) {
            enemy.flags.reset_took_damage()
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
impl std::ops::Index<RoomID> for Board {
    type Output = Room;
    fn index(&self, index: RoomID) -> &Self::Output {
        &self.rooms[index.get_inner() as usize]
    }
}
impl<T: Clone> std::ops::Index<&T> for Board
where
    Board: std::ops::Index<T>,
{
    type Output = <Board as std::ops::Index<T>>::Output;
    fn index(&self, index: &T) -> &Self::Output {
        &self[index.clone()]
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
/// The various types of maps that the board can be
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MapType {
    Normal = 0,
    Shop = 1,
}
impl FromBinary for MapType {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        let inner = u8::from_binary(binary)?;
        if inner > 1 {
            anyhow::bail!("Attempted to load MapType with illegal discriminant: {inner}")
        }
        Ok(unsafe { std::mem::transmute(inner) })
    }
}
impl ToBinary for MapType {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        unsafe { std::mem::transmute::<Self, u8>(*self) }.to_binary(binary)
    }
}
pub struct RenderSpecial {
    pub position: Vector<usize>,
    pub ch: char,
    pub style: Option<abes_nice_things::Style>,
}
impl ToBinary for RenderSpecial {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        self.position.to_binary(binary)?;
        self.ch.to_binary(binary)?;
        self.style.as_ref().to_binary(binary)
    }
}
impl FromBinary for RenderSpecial {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            position: <Vector<usize>>::from_binary(binary)?,
            ch: char::from_binary(binary)?,
            style: <Option<abes_nice_things::Style>>::from_binary(binary)?,
        })
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
    let board = Board::new(AxisLength::Small, Vector::new(0, 0), MapType::Normal).unwrap(); // 64 x 64

    for x in 0..64_usize {
        for y in 0..64_usize {
            //println!("Getting {x}, {y}");
            board[Vector::new(x, y)];
        }
    }
}
