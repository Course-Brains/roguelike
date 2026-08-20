use super::VTable;
pub static VTABLE: VTable = VTable {
    starting_health: 100000,
    kill_energy: 1000,
    init: VTable::DEFAULT_INIT,
    think: |_, _| {},
    damage: VTable::DEFAULT_DAMAGE,
    promote_tier: None,
};
