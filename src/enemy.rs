// Place mod for enemies here
pub mod basic;
pub mod dummy;
// Put the vtable here
static VTABLES: [VTable; 2] = [dummy::VTABLE, basic::VTABLE];
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
// Register the vtable here, make sure you correctly put its index
pub enum VTableID {
    Dummy = 0,
    Basic = 1,
}

use crate::Vector;
use crate::board::Board;
use crate::board::EnemyID;
use crate::math::Direction;
use crate::state::*;
use abes_nice_things::PrimAs;
use abes_nice_things::Style;
use std::any::Any;

#[derive(Debug)]
pub struct Enemy {
    /// The state which the logic can read and write to
    state: Box<dyn Any + Send>,
    /// The number of base hits required to kill it
    health: usize,
    /// The position of the enemy on the map. DO NOT INTERACT WITH THIS DIRECTLY
    position: Vector<usize>,
    /// Where the enemy is currently pathing towards
    pub move_target: Option<Vector<usize>>,
    /// The end goal of the inter room pathing. This is where the enemy eventually wants to end up
    pub end_goal: Option<Vector<usize>>,
    /// The vtable holding function pointers to the logic and enemy type specific constants
    vtable_id: VTableID,
    /// Various pieces of data which are tied to this specific instance and can spply to any enemy
    pub flags: Flags,
    /// The position used in intra room pathfinding
    logical_position: Vector<f64>,
    windup_time: usize,
}
impl Enemy {
    pub fn new(vtable_id: VTableID, position: Vector<usize>) -> Enemy {
        let vtable = vtable_id.get_vtable();
        Enemy {
            state: (vtable.init)(),
            health: vtable.starting_health,
            position,
            move_target: None,
            end_goal: None,
            vtable_id,
            flags: Flags::new(),
            logical_position: position.prim_as() + 0.5,
            windup_time: 0,
        }
    }
    pub fn render(&self) -> (char, Style) {
        let mut style = Style::new();

        // Foreground

        // These are mutually exclusive because bosses skip detection checks and are always awake
        if self.get_vtable().is_boss {
            // Bosses are blue
            style.blue();
        } else if self.flags.is_awake() {
            // Awake are yellow
            style.yellow();
        }

        // Background
        self.flags.get_windup().get_style(&mut style);

        (self.get_vtable().render_char, style)
    }
    pub fn get_vtable(&self) -> &'static VTable {
        self.vtable_id.get_vtable()
    }
    pub fn intra_room_pathfind(state: &mut State, id: EnemyID) {
        let this = state.board.get_enemy_mut(id).as_mut().unwrap();
        if this.logical_position.prim_as() != this.position {
            this.logical_position =
                PrimAs::<Vector<f64>>::prim_as(this.position) + Vector::new(0.5, 0.5);
        }
        // If we aren't moving then we don't need to pathfind
        if this.move_target.is_none() {
            return;
        }

        let logical_move_target = PrimAs::<Vector<f64>>::prim_as(this.move_target.unwrap()) + 0.5;

        let diff = logical_move_target - this.logical_position;

        // If we are close then let's do the cheaper but less pretty pathfinding
        // Additionally do the cheap one if it is a straight line to the target
        let mut dir = if this.position.is_near(this.move_target.unwrap(), 3)
            || this.position.x == this.move_target.unwrap().x
            || this.position.y == this.move_target.unwrap().y
        {
            Direction::from_vector(diff).unwrap()
        } else {
            // We can't do the cheap pathfinding :(

            let target = Vector::new(
                (this.logical_position.x + (diff.x.signum() / 2.0)).round(),
                (this.logical_position.y + (diff.y.signum() / 2.0)).round(),
            );

            let dist_to_target = target - this.logical_position;

            let effective_dist_to_target = dist_to_target / diff;
            assert!(
                effective_dist_to_target.x.is_sign_positive() || effective_dist_to_target.x == 0.0
            );
            assert!(
                effective_dist_to_target.y.is_sign_positive() || effective_dist_to_target.y == 0.0
            );

            // Horizontal movement
            if effective_dist_to_target.x < effective_dist_to_target.y {
                this.logical_position.x = target.x;
                this.logical_position.y += diff.y * effective_dist_to_target.x;
                if diff.x.is_sign_positive() {
                    Direction::Right
                } else {
                    Direction::Left
                }
            }
            // Vertical movement
            else {
                this.logical_position.y = target.y;
                this.logical_position.x += diff.x * effective_dist_to_target.y;
                if diff.y.is_sign_positive() {
                    Direction::Down
                } else {
                    Direction::Up
                }
            }
        };

        // Handling backup move direction
        // Yes this will cause desync between the logical and actual position, I do not care (it
        // fixes itself immediately)
        let position = state.board[id].as_ref().unwrap().position;
        // Fallback direction calculation
        if !Board::enemy_can_move(state, position, dir) {
            // Getting next best direction
            let remaining = *diff.clone().zero_axis(dir.axis());
            let new_dir = Direction::from_vector(remaining);
            if let Some(direction) = new_dir
                && Board::enemy_can_move(state, position, direction)
            {
                dir = direction;
            // The only case where the previous check will fail is if it is a straight line to
            // the target but it is blocked, so we are going to move around it hopefully
            } else {
                let valid_rooms = state.board.get_possible_room_ids_at_position(
                    state.board[id].as_ref().unwrap().move_target.unwrap(),
                );
                if let Some(new_dir) = [
                    Direction::Up,
                    Direction::Down,
                    Direction::Left,
                    Direction::Right,
                ]
                .into_iter()
                .filter(|possible_direction| {
                    *possible_direction != dir
                        && *possible_direction != !dir
                        && Some(*possible_direction) != new_dir
                        && state
                            .board
                            .get_room_id_of_coord(position + *possible_direction)
                            .is_some_and(|room_id| valid_rooms.contains(&room_id))
                        && Board::enemy_can_move(state, position, *possible_direction)
                })
                .next()
                {
                    dir = new_dir;
                } else {
                    // We couldn't find a new direction to move so we will give up
                    // ...
                    // for now
                    return;
                }
            }
        }

        Enemy::move_position(state, id, state.board[id].as_ref().unwrap().position + dir);
        let this = state.board.get_enemy_mut(id).as_mut().unwrap();
        if this.position == this.move_target.unwrap() {
            this.move_target = None;
        }
    }
    /// The proper way to move the enemy, this is needed because if you don't use it then the room
    /// enemy memoization can desync
    pub fn move_position(state: &mut State, id: EnemyID, new_pos: Vector<usize>) {
        let prev_rooms = state
            .board
            .get_possible_room_ids_at_position(state.board[id].as_ref().unwrap().position);
        assert!(!prev_rooms.is_empty());

        state.board[id].as_mut().unwrap().position = new_pos;

        let new_rooms = state
            .board
            .get_possible_room_ids_at_position(state.board[id].as_ref().unwrap().position);
        assert!(!new_rooms.is_empty());

        // We have to rememoize the position
        if prev_rooms != new_rooms {
            // First we remove all the old data
            for prev_room in prev_rooms.iter() {
                let prev_room = state.board.get_room_mut(*prev_room);
                for index in 0..prev_room.enemies.len() {
                    if prev_room.enemies[index] == id {
                        prev_room.enemies.swap_remove(index);
                        break;
                    }
                }
            }
            // Then we put in the new
            for new_room in new_rooms.iter() {
                let new_room = state.board.get_room_mut(*new_room);
                new_room.enemies.push(id);
            }
        }
    }
    pub fn inital_room_memoize(board: &mut Board, id: EnemyID) {
        let rooms = board.get_possible_room_ids_at_position(board[id].as_ref().unwrap().position);
        for room in rooms.iter() {
            let room = board.get_room_mut(*room);
            room.enemies.push(id);
        }
    }
    pub fn get_position(&self) -> Vector<usize> {
        self.position
    }
    /// Returns None if it was unable to figure out a possible enemy type to meet restrictions
    pub fn pick_vtable_from_budget(
        budget: &mut usize,
        max_tier: Option<usize>,
    ) -> Option<VTableID> {
        if *budget < VTableID::Basic.get_vtable().budget_cost {
            None
        } else {
            *budget -= VTableID::Basic.get_vtable().budget_cost;
            Some(VTableID::Basic)
        }
    }
}
/// Where enemy type specific logic is stored as well as some constants
#[derive(Clone, Copy, Debug)]
pub struct VTable {
    starting_health: usize,
    /// The character used to represent this enemy type during rendering
    render_char: char,
    /// Whether or not to render this as a boss, this does not affect logic in any way
    is_boss: bool,
    /// The function which initializes the state of the enemy. If the enemy does not need a state
    /// then simply give it Box<()> which won't allocate anything
    init: fn() -> Box<dyn Any + Send>,
    /// The main logic function which is called for all enemies every turn before other logic
    pub think: fn(&mut State, EnemyID),
    /// How damage is dealt to enemies. It returns if the enemy should be deleted
    pub damage: fn(&mut State, EnemyID, usize) -> bool,
    budget_cost: usize,
    pub tier: usize,
}
impl VTable {
    const DEFAULT_INIT: fn() -> Box<dyn Any + Send> = || Box::new(());
    const DEFAULT_DAMAGE: fn(&mut State, EnemyID, usize) -> bool = |state, id, damage| {
        let this = state.board.get_enemy_mut(id).as_mut().unwrap();
        if damage >= this.health {
            *state.board.get_enemy_mut(id) = None;
            return true;
        }
        this.flags.wake();
        this.health -= damage;
        false
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Flags(u8);
// 0b0000_1000
//   |||| |||+- Whether or not it is awake
//   |||| |++-- WindupState
//   |||| +---- Whether or not to do pathfinding
//   |||+------ Unassigned
//   ||+------- Unassigned
//   |+-------- Unassigned
//   +--------- Unassigned
impl Flags {
    fn new() -> Flags {
        Flags(0b0000_1000)
    }
    pub fn is_awake(&self) -> bool {
        (self.0 & 0b1) != 0
    }
    pub fn wake(&mut self) {
        self.0 |= 0b1
    }
    pub fn set_windup(&mut self, state: WindupState) {
        self.0 &= !WindupState::MASK; // clear the windup bits
        self.0 |= unsafe { std::mem::transmute::<WindupState, u8>(state) };
    }
    pub fn get_windup(&self) -> WindupState {
        let windup_bits = self.0 & WindupState::MASK;
        debug_assert_ne!(windup_bits, 0b0110);
        unsafe { std::mem::transmute(windup_bits) }
    }
    /// Returns if the enemy is in ANY windup state
    pub fn is_windup(&self) -> bool {
        (self.0 & 0b0110) != 0
    }
    pub fn set_pathing(&mut self, should_path: bool) {
        if should_path {
            self.0 |= 0b0000_1000
        } else {
            self.0 &= 0b1111_0111
        }
    }
    pub fn should_path(&self) -> bool {
        (self.0 & 0b1000) != 0
    }
}
#[repr(u8)]
enum WindupState {
    None = 0b0000,
    Physical = 0b0010,
    Magical = 0b0100,
    // Unassigned = 0b0110
    // If you decide to add a third windup state later then modify Flags::get_windup because it
    // will panic otherwise
}
impl WindupState {
    const MASK: u8 = 0b0000_0110;
    fn get_style(&self, style: &mut Style) {
        match self {
            WindupState::Physical => {
                style.background_red();
            }
            WindupState::Magical => {
                style.background_purple();
            }
            WindupState::None => {}
        }
    }
    fn is_none(&self) -> bool {
        matches!(self, WindupState::None)
    }
    fn is_physical(&self) -> bool {
        matches!(self, WindupState::Physical)
    }
    fn is_magical(&self) -> bool {
        matches!(self, WindupState::Magical)
    }
}
impl VTableID {
    pub fn get_vtable(self) -> &'static VTable {
        &VTABLES[self.to_inner() as usize]
    }
    fn to_inner(self) -> u8 {
        unsafe { std::mem::transmute(self) }
    }
}
impl PartialOrd for VTableID {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.to_inner().partial_cmp(&other.to_inner())
    }
}
impl Ord for VTableID {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_inner().cmp(&other.to_inner())
    }
}
