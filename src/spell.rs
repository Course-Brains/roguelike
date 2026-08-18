// Each spell can be cast manually or put on a spell circle.
// Contact spells require you to be able to touch the target
// Position spells only need a target position
// Contact spells when on a spell circle will wait for something to walk over them before
// triggering
// Position spells when on a spell circle will cast repeatedly while active

mod fireball;

use crate::math::*;
use crate::state::Entity;
use crate::state::State;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;
use std::io::Read;
use std::io::Write;

////////////////////
// Spell registry //
////////////////////

/// A spell which targets a position. When in a spell circle it will cast at a known position
/// repeatedly
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum PositionSpell {
    Fireball = 0,
}

/// A spell which requires contact with the target [Entity]. When in a spell circle it will wait
/// for an [Entity] to walk over it before casting
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
//#[repr(u8)]
pub enum ContactSpell {}

////////////////////////////////////////////////////////////
// Things which do need changing when you add a new spell //
////////////////////////////////////////////////////////////

impl PositionSpell {
    const HIGHEST_DISCRIMINANT: u8 = 0;
    pub fn get_name(self) -> &'static str {
        match self {
            Self::Fireball => "fireball",
        }
    }
    pub fn minimum_mana(self) -> usize {
        match self {
            Self::Fireball => 5,
        }
    }
    pub fn cast(
        self,
        state: &mut State,
        origin: Vector<usize>,
        _caster: Entity,
        target: Vector<usize>,
        energy_used: usize,
    ) {
        match self {
            Self::Fireball => fireball::cast(state, origin, target, energy_used),
        }
    }
}

impl ContactSpell {
    pub fn get_name(self) -> &'static str {
        match self {}
    }
    pub fn minimum_mana(self) -> usize {
        match self {}
    }
    pub fn cast(self, _state: &mut State, _caster: Entity, _target: Entity) {
        match self {}
    }
}

///////////////////////////////////////////////////////////////
// Things which don't need changing when you add a new spell //
///////////////////////////////////////////////////////////////
impl PositionSpell {
    pub fn to_inner(self) -> u8 {
        unsafe { std::mem::transmute(self) }
    }
    pub fn from_inner(inner: u8) -> Result<PositionSpell> {
        if inner > Self::HIGHEST_DISCRIMINANT {
            anyhow::bail!(
                "Attempted to create PositionSpell with invalid discriminant:\n\
            Tried to use discriminant {inner} when highest valid is {}",
                Self::HIGHEST_DISCRIMINANT
            )
        }
        Ok(unsafe { std::mem::transmute(inner) })
    }
}
impl ToBinary for PositionSpell {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        self.to_inner().to_binary(binary)
    }
}
impl FromBinary for PositionSpell {
    fn from_binary(binary: &mut dyn Read) -> Result<Self> {
        Self::from_inner(u8::from_binary(binary)?)
    }
}
impl ToBinary for ContactSpell {
    fn to_binary(&self, _binary: &mut dyn Write) -> Result<()> {
        anyhow::bail!("Something has gone very wrong")
    }
}
impl FromBinary for ContactSpell {
    fn from_binary(_binary: &mut dyn Read) -> Result<Self> {
        anyhow::bail!("Something has gone very wrong")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spell {
    Position(PositionSpell),
    Contact(ContactSpell),
}
impl ToBinary for Spell {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        match self {
            Spell::Position(spell) => {
                false.to_binary(binary)?;
                spell.to_binary(binary)
            }
            Spell::Contact(spell) => {
                true.to_binary(binary)?;
                spell.to_binary(binary)
            }
        }
    }
}
impl FromBinary for Spell {
    fn from_binary(binary: &mut dyn Read) -> Result<Self> {
        Ok(match bool::from_binary(binary)? {
            false => Spell::Position(PositionSpell::from_binary(binary)?),
            true => Spell::Contact(ContactSpell::from_binary(binary)?),
        })
    }
}
impl Spell {
    pub fn get_name(&self) -> &'static str {
        match self {
            Self::Position(position) => position.get_name(),
            Self::Contact(contact) => contact.get_name(),
        }
    }
    pub fn minimum_mana(&self) -> usize {
        match self {
            Self::Position(position) => position.minimum_mana(),
            Self::Contact(contact) => contact.minimum_mana(),
        }
    }
    /// Returns position for position spells and contact for contact spells
    pub fn spell_type_name(&self) -> &'static str {
        match self {
            Self::Position(_) => "position",
            Self::Contact(_) => "contact",
        }
    }
}
