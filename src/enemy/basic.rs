use super::VTable;
pub static VTABLE: VTable = VTable {
    starting_health: 3,
    render_char: '1',
    is_boss: false,
    init: VTable::DEFAULT_INIT,
    think,
    damage: VTable::DEFAULT_DAMAGE,
};
fn think(state: &mut crate::state::State, id: super::EnemyID) {
    // If we aren't awake then try to wake up
    if !state.board[id].as_ref().unwrap().flags.is_awake() {
        todo!()
    }
    todo!()
}
