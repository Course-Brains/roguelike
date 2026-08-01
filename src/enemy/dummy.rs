use super::VTable;
pub static VTABLE: VTable = VTable {
    starting_health: 1000,
    render_char: '0',
    is_boss: false,
    init: VTable::DEFAULT_INIT,
    think: |_, _| {},
    damage: VTable::DEFAULT_DAMAGE,
    budget_cost: 0,
    tier: 9999,
};
