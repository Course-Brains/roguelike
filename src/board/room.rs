use crate::math::Vector;
use crate::math::Zone;

pub struct RoomID(usize);
pub fn room_id(internal: usize) -> RoomID {
    RoomID(internal)
}
pub struct Room {
    connections: Vec<(Vector<usize>, RoomID)>,
    bounds: Zone<usize>,
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
