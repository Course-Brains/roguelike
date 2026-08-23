//! Boons which are spawned in rooms to help the player but increase the budget of that room when
//! spawned

use crate::board::Tile;
use crate::board::WalkTrigger;
use crate::random::PickRandom;
use crate::random::Random;
use crate::state::State;
use abes_nice_things::Style;
use abes_nice_things::{FromBinary, ToBinary};

pub struct Boon {
    pub name: &'static str,
    pub waila: &'static str,
    pub budget: usize,
    /// The rendering to be used before the boon is used
    pub unused_render: (char, Option<Style>),
    /// The rendering to be used after the boon is used
    pub used_render: (char, Option<Style>),
    /// What happens when the player steps on it
    pub effect: fn(&mut State),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BoonID {
    Fountain = 0,
}

static BOONS: &[Boon] = &[
    // 0: Fountain of healing
    Boon {
        name: "Fountain of healing",
        waila: "A fountain which will heal you once for 4-32",
        budget: 5,
        unused_render: ('F', Some(*Style::new().blue())),
        used_render: ('F', None),
        effect: |state| {
            state
                .player
                .heal((((u8::random() & 0b111) + 1) << 2) as usize)
        },
    },
];

// End of stuff to change for adding a new boon //

impl ToBinary for BoonID {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> anyhow::Result<()> {
        self.to_inner().to_binary(binary)
    }
}
impl FromBinary for BoonID {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> anyhow::Result<Self> {
        let inner = u8::from_binary(binary)?;
        if inner as usize >= BOONS.len() {
            anyhow::bail!("Attempted to load invalid boon id {inner}");
        }
        Ok(unsafe { std::mem::transmute(inner) })
    }
}
impl BoonID {
    fn to_inner(self) -> u8 {
        unsafe { std::mem::transmute(self) }
    }
    fn to_index(self) -> usize {
        self.to_inner() as usize
    }
    pub fn get_boon(self) -> &'static Boon {
        &BOONS[self.to_index()]
    }
    pub fn to_tile(self) -> Tile {
        Tile::WalkTrigger(WalkTrigger::Boon(self, false))
    }
}
impl Random for BoonID {
    fn random() -> Self {
        unsafe { std::mem::transmute((..BOONS.len()).generate() as u8) }
    }
}
