use crate::board::EnemyID;
use crate::board::MapType;
use crate::math::*;
use crate::state::State;
use abes_nice_things::{FromBinary, ToBinary};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WalkTrigger {
    Exit,
}
impl ToBinary for WalkTrigger {
    fn to_binary(&self, _binary: &mut dyn std::io::prelude::Write) -> anyhow::Result<()> {
        Ok(())
    }
}
impl FromBinary for WalkTrigger {
    fn from_binary(_binary: &mut dyn std::io::prelude::Read) -> anyhow::Result<Self> {
        Ok(WalkTrigger::Exit)
    }
}
impl WalkTrigger {
    pub fn get_char(&self) -> char {
        '∏'
    }
    /// Handle the case of a player walking on the trigger. It returns if this should be deleted
    /// afterwards
    pub fn handle_player(self, state: &mut State) -> bool {
        match state.board.map_type {
            // Go to shop
            MapType::Normal => state.go_to_shop(),
            // Go to next level
            MapType::Shop => {
                state.board = state.next_level.take().unwrap().1.unwrap().unwrap();
                state.player.position = Vector::new(1, 1)
            }
        }
        false
    }
    /// Handle the case of an enemy walking on the trigger, returns if this should be deleted
    /// afterwards
    pub fn handle_enemy(self, _state: &mut State, _enemy: EnemyID) -> bool {
        false
    }
}
