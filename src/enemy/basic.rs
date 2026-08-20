use super::VTable;
use super::WindupState;
use crate::board::Board;
use crate::math::*;
use crate::random::Random;
use crate::state::*;
pub static VTABLE: VTable = VTable {
    starting_health: 30,
    kill_energy: 1,
    init: VTable::DEFAULT_INIT,
    think,
    damage: VTable::DEFAULT_DAMAGE,
    promote_tier: Some((1, 0, super::VTableID::AliceBoss)),
};
const SMACK_RANGE: usize = 1;
fn think(state: &mut State, id: super::EnemyID) {
    // If we aren't awake then try to wake up
    if !state.board[id].as_ref().unwrap().flags.is_awake() {
        let mut wake_distance = (u8::random() & 0b111) as usize;
        if !state
            .board
            .get_possible_room_ids_at_position(state.board[id].as_ref().unwrap().get_position())
            .iter()
            .any(|room_id| {
                state
                    .board
                    .get_possible_room_ids_at_position(state.player.position)
                    .contains(room_id)
            })
        {
            // We are not in the same room :(
            wake_distance /= 2;
        }
        if state.board[id]
            .as_ref()
            .unwrap()
            .get_position()
            .is_near(state.player.position, wake_distance)
        {
            // Wakey wakey
            state.board[id].as_mut().unwrap().flags.wake()
        }
    }

    // Since we are awake let's get killing
    let this = state.board[id].as_mut().unwrap();
    // If we are confused then walk in random direction if able
    if this.effects.has(crate::effect::EffectID::Confusion) {
        this.windup_time = 0;
        this.flags.set_pathing(true);
        this.flags.set_windup(WindupState::None);
        let dir = Direction::random();
        let start = this.get_position();
        state.board[id].as_mut().unwrap().end_goal = if Board::enemy_can_move(state, start, dir) {
            Some(start + dir)
        } else {
            None
        };
        return;
    }
    this.end_goal = Some(state.player.position);

    // Are we smacking?
    if this.flags.get_windup().is_physical() {
        this.windup_time -= 1;
        // Smack o clock
        if this.windup_time == 0 {
            this.flags.set_windup(WindupState::None);
            if state.player.position.is_near(this.position, SMACK_RANGE) {
                crate::player::Player::damage(state, (u8::random() & 0b111) as usize + 1);
                return;
            }
        }
    }
    // do we even want to smack?
    else {
        if state.player.position.is_near(this.position, SMACK_RANGE) {
            this.flags.set_pathing(false);
            this.flags.set_windup(WindupState::Physical);
            this.windup_time = 2;
        } else {
            this.flags.set_pathing(true);
        }
    }
}
