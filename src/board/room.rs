use super::EnemyID;
use crate::math::Vector;
use crate::math::Zone;
use abes_nice_things::PrimAs;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;
use std::num::NonZeroU16;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct RoomID(u16);
impl RoomID {
    pub fn get_inner(self) -> u16 {
        self.0
    }
}
pub fn room_id<T: PrimAs<u16>>(internal: T) -> RoomID {
    RoomID(internal.prim_as())
}
impl ToBinary for RoomID {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        self.get_inner().to_binary(binary)
    }
}
impl FromBinary for RoomID {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(RoomID(u16::from_binary(binary)?))
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Change what this stores at your own risk
pub struct Room {
    pub connections: Vec<(Vector<usize>, RoomID)>,
    pub bounds: Zone<usize>,
    pub enemies: Vec<EnemyID>,
}
impl ToBinary for Room {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        self.connections.len().to_binary(binary)?;
        for (position, room) in self.connections.iter() {
            position.to_binary(binary)?;
            room.to_binary(binary)?;
        }
        self.bounds.to_binary(binary)?;
        self.enemies.to_binary(binary)
    }
}
impl FromBinary for Room {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(Room {
            connections: <Vec<(Vector<usize>, RoomID)>>::from_binary(binary)?,
            bounds: <Zone<usize>>::from_binary(binary)?,
            enemies: <Vec<EnemyID>>::from_binary(binary)?,
        })
    }
}
impl Room {
    pub fn new(bounds: Zone<usize>) -> Room {
        Room {
            connections: Vec::new(),
            bounds,
            enemies: Vec::new(),
        }
    }
    pub fn add_connection(&mut self, position: Vector<usize>, connectee: RoomID) {
        self.connections.push((position, connectee));
    }
    pub fn get_bounds(&self) -> Zone<usize> {
        self.bounds
    }
}

/// A room id and various flags
///
/// This is able to hold the room id if there is one.
///
/// The benefit to using this over Option<RoomID> is that this uses less memory.
///
/// This is used so that the interiors of rooms on the board and be queried for what room they are
/// a part of
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomIDFlagged(Option<NonZeroU16>);
impl ToBinary for RoomIDFlagged {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        match self.0 {
            Some(id) => id.get(),
            None => 0,
        }
        .to_binary(binary)
    }
}
impl FromBinary for RoomIDFlagged {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(RoomIDFlagged(NonZeroU16::new(u16::from_binary(binary)?)))
    }
}
impl RoomIDFlagged {
    pub fn new(room_id: Option<RoomID>) -> RoomIDFlagged {
        RoomIDFlagged(room_id.map(|room_id| NonZeroU16::new(room_id.get_inner() + 1).unwrap()))
    }
    pub fn get_id(self) -> Option<RoomID> {
        self.0.map(|id| RoomID(id.get() - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Random;
    use std::collections::VecDeque;
    #[test]
    fn room_id_binary() {
        let mut buf = VecDeque::new();
        for _ in 0..1000 {
            let test = RoomID(u16::random());
            test.to_binary(&mut buf).unwrap();
            assert_eq!(test, RoomID::from_binary(&mut buf).unwrap());
        }
        assert_eq!(buf.len(), 0);
    }
    #[test]
    fn room_id_flagged_binary() {
        let mut buf = VecDeque::new();
        let test = RoomIDFlagged::new(None);
        test.to_binary(&mut buf).unwrap();
        assert_eq!(test, RoomIDFlagged::from_binary(&mut buf).unwrap());
        buf.truncate(0);
        for _ in 0..1000 {
            let test = RoomIDFlagged(NonZeroU16::new(u16::random()));
            test.to_binary(&mut buf).unwrap();
            assert_eq!(test, RoomIDFlagged::from_binary(&mut buf).unwrap());
        }
        assert_eq!(buf.len(), 0);
    }
}
