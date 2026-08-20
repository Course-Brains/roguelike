use abes_nice_things::{FromBinary, ToBinary};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum UpgradeID {
    Health = 0,
    Heal = 1,
    OverflowHealRate = 2,
    SelfAwareness = 3,
}

static ROOTS: &[UpgradeID] = &[UpgradeID::Health, UpgradeID::SelfAwareness];

static UPGRADES: &[Upgrade] = &[
    Upgrade {
        name: "Health",
        cost: 30,
        max_stacks: 5,
        unlocks: &[UpgradeID::Heal, UpgradeID::OverflowHealRate],
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
    Upgrade {
        name: "Self awareness",
        cost: 100,
        max_stacks: 1,
        unlocks: &[],
        effect: |_, _| {},
    },
];

impl ToBinary for UpgradeID {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> anyhow::Result<()> {
        self.to_inner().to_binary(binary)
    }
}
impl FromBinary for UpgradeID {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> anyhow::Result<Self> {
        let inner = u8::from_binary(binary)?;
        if inner as usize >= UPGRADES.len() {
            anyhow::bail!("Attempted to load UpgradeID with invalid discriminant: {inner}");
        }
        Ok(unsafe { std::mem::transmute(inner) })
    }
}

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

    for root in ROOTS.iter() {
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
    pub fn to_upgrade(self) -> &'static Upgrade {
        &UPGRADES[self.to_index()]
    }
}
pub struct Upgrade {
    pub name: &'static str,
    pub cost: usize,
    pub max_stacks: u8,
    /// The unlocks are only unlocked when the current stacks reaches the max stacks
    unlocks: &'static [UpgradeID],
    /// state and the new number of stacks
    effect: fn(&mut crate::state::State, u8),
}
pub struct Upgrades {
    purchased: [u8; UPGRADES.len()],
}
impl ToBinary for Upgrades {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> anyhow::Result<()> {
        self.purchased.to_binary(binary)
    }
}
impl FromBinary for Upgrades {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> anyhow::Result<Self> {
        Ok(Upgrades {
            purchased: <[u8; UPGRADES.len()]>::from_binary(binary)?,
        })
    }
}
impl Upgrades {
    pub fn new() -> Upgrades {
        Upgrades {
            purchased: [0; UPGRADES.len()],
        }
    }
    pub fn has_bought(&self, upgrade: UpgradeID) -> bool {
        self.purchased[upgrade.to_index()] > 0
    }
    pub fn num_stacks(&self, upgrade: UpgradeID) -> u8 {
        self.purchased[upgrade.to_index()]
    }
    pub fn has_completed(&self, upgrade: UpgradeID) -> bool {
        self.purchased[upgrade.to_index()] >= upgrade.to_upgrade().max_stacks
    }
    /// Returns if the player has bought any upgrades
    pub fn has_bought_any(&self) -> bool {
        self.purchased.iter().any(|stacks| *stacks > 0)
    }
    pub fn get_all(
        &self,
    ) -> std::iter::Map<
        std::iter::Enumerate<std::slice::Iter<'_, u8>>,
        fn((usize, &u8)) -> (UpgradeID, u8),
    > {
        self.purchased
            .iter()
            .enumerate()
            .map(|(index, stacks)| (unsafe { std::mem::transmute(index as u8) }, *stacks))
    }
    /// Runs the on purchase code of the upgrade and adds a stack BUT DOES NOT TAKE THE MONEY OR
    /// FAIL WITH INSUFFICIENT MONEY.
    ///
    /// It also does not do any checks for whther or not you will be going over the max stacks of
    /// that upgrade.
    pub fn buy(state: &mut crate::state::State, upgrade: UpgradeID) {
        state.player.upgrades.purchased[upgrade.to_index()] += 1;
        (upgrade.to_upgrade().effect)(state, state.player.upgrades.num_stacks(upgrade))
    }
}
