use crate::math::Vector;
use crate::math::Zone;
use abes_nice_things::PrimAs;
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
#[derive(Clone, Debug, PartialEq)]
/// Change what this stores at your own risk
pub struct Room {
    pub connections: Vec<(Vector<usize>, RoomID)>,
    pub bounds: Zone<usize>,
}
impl Room {
    pub fn new(bounds: Zone<usize>) -> Room {
        Room {
            connections: Vec::new(),
            bounds,
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
impl RoomIDFlagged {
    pub fn new(room_id: Option<RoomID>) -> RoomIDFlagged {
        RoomIDFlagged(room_id.map(|room_id| NonZeroU16::new(room_id.get_inner() + 1).unwrap()))
    }
    pub fn get_id(self) -> Option<RoomID> {
        self.0.map(|id| RoomID(id.get() - 1))
    }
}
