use super::VTable;
pub static VTABLE: VTable = VTable {
    starting_health: 1000,
    init: VTable::DEFAULT_INIT,
    think: |_, _| {},
    damage: VTable::DEFAULT_DAMAGE,
    budget_cost: 0,
    promote_tier: None,
};
