use crate::Vector;
use crate::board::EnemyID;
use crate::state::State;
use std::any::Any;

pub struct Enemy {
    /// The state which the logic can read and write to
    state: Box<dyn Any>,
    /// The number of base hits required to kill it
    health: usize,
    /// The position of the enemy on the map
    position: Vector<usize>,
    /// Where the enemy is currently pathing towards
    move_target: Option<Vector<usize>>,
    /// The vtable holding function pointers to the logic and enemy type specific constants
    vtable: &'static VTable,
    flags: Flags,
}
impl Enemy {
    fn new(vtable: &'static VTable, position: Vector<usize>) -> Enemy {
        Enemy {
            state: (vtable.init)(),
            health: vtable.starting_health,
            position,
            move_target: None,
            vtable: vtable,
            flags: Flags::new(),
        }
    }
}
/// Where enemy type specific logic is stored as well as some constants
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VTable {
    starting_health: usize,
    /// The function which initializes the state of the enemy. If the enemy does not need a state
    /// then simply give it Box<()> which won't allocate anything
    init: fn() -> Box<dyn Any>,
    /// The main logic function which is called for all enemies every turn before other logic
    think: fn(&mut State, EnemyID),
}

struct Flags(u8);
// 0b0000_0000
//   |||| |||+- Whether or not it is awake
//   |||| ||+-- Unassigned
//   |||| |+--- Unassigned
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
}
