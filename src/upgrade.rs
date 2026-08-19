#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum UpgradeID {
    Example = 0,
}

static UPGRADES: &[Upgrade] = &[];

pub fn get_all_available(tree: &Upgrades) -> Vec<UpgradeID> {
    let mut available = Vec::new();

    fn helper(available: &mut Vec<UpgradeID>, tree: &Upgrades, current: UpgradeID) {
        if tree.has_bought(current) {
            for next in current.to_upgrade().unlocks.iter() {
                helper(available, tree, *next);
            }
        } else {
            available.push(current)
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
    cost: usize,
    name: &'static str,
    effect: fn(&mut crate::state::State),
    unlocks: &'static [UpgradeID],
}
pub struct Upgrades {
    roots: &'static [UpgradeID],
    purchased: [bool; UPGRADES.len()],
}
impl Upgrades {
    pub fn has_bought(&self, upgrade: UpgradeID) -> bool {
        self.purchased[upgrade.to_index()]
    }
}
