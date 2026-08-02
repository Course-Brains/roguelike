use super::VTable;
use crate::RayCast;
use crate::random::Random;
use crate::state::*;
pub static VTABLE: VTable = VTable {
    starting_health: 3,
    render_char: '1',
    is_boss: false,
    init: VTable::DEFAULT_INIT,
    think,
    damage: VTable::DEFAULT_DAMAGE,
    budget_cost: 1,
    tier: 0,
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
    this.end_goal = Some(state.player.position);

    // Are we smacking?
    if this.flags.get_windup().is_physical() {
        this.windup_time -= 1;
        // Smack o clock
        if this.windup_time == 0 {
            this.flags.set_pathing(true);
            this.flags.set_windup(super::WindupState::None);
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
            this.flags.set_windup(super::WindupState::Physical);
            this.windup_time = 2;
        }
    }
}
