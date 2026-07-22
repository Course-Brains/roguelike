// Place mod for enemies here
pub mod dummy;
// CONSIDER STATIC VTABLE ARRAY TO ALLOW SAVE/LOAD OF ENEMIES AND MORE COMPACT DATA
use crate::Vector;
use crate::board::EnemyID;
use crate::math::Direction;
use crate::state::State;
use abes_nice_things::PrimAs;
use abes_nice_things::Style;
use std::any::Any;

pub struct Enemy {
    /// The state which the logic can read and write to
    state: Box<dyn Any + Send>,
    /// The number of base hits required to kill it
    health: usize,
    /// The position of the enemy on the map
    pub position: Vector<usize>,
    /// Where the enemy is currently pathing towards
    pub move_target: Option<Vector<usize>>,
    /// The vtable holding function pointers to the logic and enemy type specific constants
    vtable: &'static VTable,
    /// Various pieces of data which are tied to this specific instance and can spply to any enemy
    flags: Flags,
    /// The position used in intra room pathfinding
    logical_position: Vector<f64>,
}
impl Enemy {
    pub fn new(vtable: &'static VTable, position: Vector<usize>) -> Enemy {
        Enemy {
            state: (vtable.init)(),
            health: vtable.starting_health,
            position,
            move_target: None,
            vtable: vtable,
            flags: Flags::new(),
            logical_position: position.prim_as() + 0.5,
        }
    }
    pub fn render(&self) -> (char, Style) {
        let mut style = Style::new();

        // Foreground

        // These are mutually exclusive because bosses skip detection checks and are always awake
        if self.vtable.is_boss {
            // Bosses are blue
            style.blue();
        } else if self.flags.is_awake() {
            // Awake are yellow
            style.yellow();
        }

        // Background
        self.flags.get_windup().get_style(&mut style);

        (self.vtable.render_char, style)
    }
    pub fn get_vtable(&self) -> &'static VTable {
        self.vtable
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
        if this.position.is_near(this.move_target.unwrap(), 3)
            || this.position.x == this.move_target.unwrap().x
            || this.position.y == this.move_target.unwrap().y
        {
            this.flags.set_windup(WindupState::Physical);
            let mut move_dir = Direction::from_vector(diff).unwrap();
            let position = this.position;

            // Fallback direction calculation
            if !state.board.enemy_can_move(position, move_dir) {
                // Getting next best direction
                let remaining = *diff.clone().zero_axis(move_dir.axis());
                if let Some(direction) = Direction::from_vector(remaining)
                    && state.board.enemy_can_move(position, direction)
                {
                    move_dir = direction;
                } else {
                    return;
                }
            }

            // Yes this is needed
            let this = state.board.get_enemy_mut(id).as_mut().unwrap();
            // Actually moving
            this.position += move_dir;
            if this.position == this.move_target.unwrap() {
                this.move_target = None;
            }
            return;
        }
        this.flags.set_windup(WindupState::Magical);

        // We can't do the cheap pathfinding :(

        let target = Vector::new(
            (this.logical_position.x + (diff.x.signum() / 2.0)).round(),
            (this.logical_position.y + (diff.y.signum() / 2.0)).round(),
        );

        let dist_to_target = target - this.logical_position;

        let effective_dist_to_target = dist_to_target / diff;
        assert!(effective_dist_to_target.x.is_sign_positive() || effective_dist_to_target.x == 0.0);
        assert!(effective_dist_to_target.y.is_sign_positive() || effective_dist_to_target.y == 0.0);

        let mut dir =
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
            };

        // Handling backup move direction
        let position = this.position;
        if !state.board.enemy_can_move(position, dir) {
            // Getting next best direction
            let remaining = *diff.clone().zero_axis(dir.axis());
            if let Some(direction) = Direction::from_vector(remaining)
                && state.board.enemy_can_move(position, direction)
            {
                dir = direction;
            } else {
                return;
            }
        }

        let this = state.board.get_enemy_mut(id).as_mut().unwrap();
        this.position += dir;
        if this.position == this.move_target.unwrap() {
            this.move_target = None;
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
}
impl VTable {
    const DEFAULT_INIT: fn() -> Box<dyn Any + Send> = || Box::new(());
    const DEFAULT_DAMAGE: fn(&mut State, EnemyID, usize) -> bool = |state, id, damage| {
        let this = state.board.get_enemy_mut(id).as_mut().unwrap();
        if damage >= this.health {
            return true;
        }
        this.health -= damage;
        false
    };
}

struct Flags(u8);
// 0b0000_0000
//   |||| |||+- Whether or not it is awake
//   |||| |++-- WindupState
//   |||| +---- Unassigned
//   |||+------ Unassigned
//   ||+------- Unassigned
//   |+-------- Unassigned
//   +--------- Unassigned
impl Flags {
    fn new() -> Flags {
        Flags(0b0000_0000)
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
}
