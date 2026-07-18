// Place mod for enemies here
pub mod dummy;

use crate::Vector;
use crate::board::EnemyID;
use crate::state::State;
use abes_nice_things::Style;
use std::any::Any;

pub struct Enemy {
    /// The state which the logic can read and write to
    state: Box<dyn Any>,
    /// The number of base hits required to kill it
    health: usize,
    /// The position of the enemy on the map
    pub position: Vector<usize>,
    /// Where the enemy is currently pathing towards
    move_target: Option<Vector<usize>>,
    /// The vtable holding function pointers to the logic and enemy type specific constants
    vtable: &'static VTable,
    /// Various pieces of data which are tied to this specific instance and can spply to any enemy
    flags: Flags,
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
        }
    }
    pub fn render(&self) -> (char, Option<Style>) {
        (
            self.vtable.render_char,
            if self.flags.is_windup() {
                let mut style = Style::new();
                self.flags.get_windup().get_style(&mut style);
                Some(style)
            } else {
                None
            },
        )
    }
    pub fn get_vtable(&self) -> &'static VTable {
        self.vtable
    }
}
/// Where enemy type specific logic is stored as well as some constants
#[derive(Clone, Copy, Debug)]
pub struct VTable {
    pub starting_health: usize,
    /// The character used to represent this enemy type during rendering
    pub render_char: char,
    /// Whether or not to render this as a boss, this does not affect logic in any way
    pub is_boss: bool,
    /// The function which initializes the state of the enemy. If the enemy does not need a state
    /// then simply give it Box<()> which won't allocate anything
    pub init: fn() -> Box<dyn Any>,
    /// The main logic function which is called for all enemies every turn before other logic
    pub think: fn(&mut State, EnemyID),
    /// How damage is dealt to enemies. It returns if the enemy should be deleted
    pub damage: fn(&mut State, EnemyID, usize) -> bool,
}
impl VTable {
    pub const DEFAULT_INIT: fn() -> Box<dyn Any> = || Box::new(());
    pub const DEFAULT_DAMAGE: fn(&mut State, EnemyID, usize) -> bool = |state, id, damage| {
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
