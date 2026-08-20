#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum UpgradeID {
    HealthBoost = 0,
    HealBoost = 1,
    OverflowHealRate = 2,
}

static UPGRADES: &[Upgrade] = &[
    Upgrade {
        name: "Health",
        cost: 30,
        max_stacks: 5,
        unlocks: &[UpgradeID::HealBoost, UpgradeID::OverflowHealRate],
        effect: |state, _| state.player.max_health += (state.player.max_health / 5).max(1),
    },
    Upgrade {
        name: "Heal",
        cost: 50,
        max_stacks: 10,
        unlocks: &[],
        effect: |state, _| state.player.heal_mult += 0.3,
    },
    Upgrade {
        name: "Overflow heal rate",
        cost: 100,
        max_stacks: 3,
        unlocks: &[],
        effect: |state, _| {
            state.player.overflow_health_per_energy +=
                (state.player.overflow_health_per_energy / 2).max(1)
        },
    },
];

/// Gets the id and stacks remaining
pub fn get_all_available(tree: &Upgrades) -> Vec<(UpgradeID, u8)> {
    let mut available = Vec::new();

    fn helper(available: &mut Vec<(UpgradeID, u8)>, tree: &Upgrades, current: UpgradeID) {
        if tree.has_completed(current) {
            for next in current.to_upgrade().unlocks.iter() {
                helper(available, tree, *next);
            }
        } else {
            available.push((
                current,
                current.to_upgrade().max_stacks - tree.num_stacks(current),
            ))
        }
    }

    for root in tree.roots.iter() {
        helper(&mut available, tree, *root);
    }
    available
}
impl UpgradeID {
    fn to_inner(self) -> u8 {
        unsafe { std::mem::transmute(self) }
    }
    fn to_index(self) -> usize {
        self.to_inner() as usize
    }
    fn to_upgrade(self) -> &'static Upgrade {
        &UPGRADES[self.to_index()]
    }
}
pub struct Upgrade {
    name: &'static str,
    cost: usize,
    max_stacks: u8,
    // The unlocks are only unlocked when the current stacks reaches the max stacks
    unlocks: &'static [UpgradeID],
    // state and the new number of stacks
    effect: fn(&mut crate::state::State, u8),
}
pub struct Upgrades {
    roots: &'static [UpgradeID],
    purchased: [u8; UPGRADES.len()],
}
impl Upgrades {
    pub fn has_bought(&self, upgrade: UpgradeID) -> bool {
        self.purchased[upgrade.to_index()] > 0
    }
    pub fn num_stacks(&self, upgrade: UpgradeID) -> u8 {
        self.purchased[upgrade.to_index()]
    }
    pub fn has_completed(&self, upgrade: UpgradeID) -> bool {
        self.purchased[upgrade.to_index()] == upgrade.to_upgrade().max_stacks
    }
}
