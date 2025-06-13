use std::ops::Range;
use std::thread::JoinHandle;
use std::rc::Rc;
use std::cell::RefCell;
use crate::{Board, random, Vector, pieces::door::Door, board::Piece};
use albatrice::debug;
const INTERVAL: usize = 5;
const MINIMUM: usize = 10;
pub fn generate(x: usize, y: usize, render_x: usize, render_y: usize) -> JoinHandle<Board> {
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let mut room = Room::new(0..(x-1), 0..(y-1));
        room.subdivide();
        room.remove_extra_adjacents();
        let mut board = Board::new(x, y, render_x, render_y);
        room.create_map_rooms(&mut board);
        room.place_doors(&mut board);
        let elapsed = start.elapsed();
        crate::log!("Map gen time: {}({})", elapsed.as_millis(), elapsed.as_nanos());
        board
    })
}
type Adjacent = Vec<Rc<RefCell<Room>>>;
struct Room {
    x_bounds: Range<usize>,
    y_bounds: Range<usize>,
    sub_room1: Option<Rc<RefCell<Room>>>,
    sub_room2: Option<Rc<RefCell<Room>>>,
    up: Adjacent,
    down: Adjacent,
    left: Adjacent,
    right: Adjacent
    
}
impl Room {
    fn new(x_bounds: Range<usize>, y_bounds: Range<usize>) -> Room {
        Room {
            x_bounds,
            y_bounds,
            sub_room1: None,
            sub_room2: None,
            up: Vec::new(),
            down: Vec::new(),
            left: Vec::new(),
            right: Vec::new(),
        }
    }
    fn subdivide(&mut self) {
        let x_len = self.x_bounds.end - self.x_bounds.start;
        let y_len = self.y_bounds.end - self.y_bounds.start;
        debug!({
            assert_eq!(x_len%INTERVAL, 0, "x_len = {x_len}");
            assert_eq!(y_len%INTERVAL, 0, "y_len = {y_len}");
        });
        if x_len <= MINIMUM || x_len <= MINIMUM { return }
        let mut axis;
        if x_len > y_len {
            axis = Axis::Vertical;
        }
        else {
            axis = Axis::Horizontal;
        }
        if random()&0b0011_1111 == 0 {
            // 1 in 64 to do the other axis instead
            axis = !axis;
        }
        let axis_len = match axis {
            Axis::Vertical => x_len,
            Axis::Horizontal => y_len,
        };
        let axis_bounds = match axis {
            Axis::Vertical => &self.x_bounds,
            Axis::Horizontal => &self.y_bounds,
        };
        let num_splits = axis_len/INTERVAL - 2;
        if num_splits == 0 { return }
        let split_point = (random()%num_splits as u8 + 1) as usize *INTERVAL + axis_bounds.start;
        debug!({
            assert!(split_point > axis_bounds.start);
            assert!(split_point < axis_bounds.end);
        });
        if split_point - axis_bounds.start < MINIMUM { return }
        if axis_bounds.end - split_point < MINIMUM { return }
        match axis {
            Axis::Vertical => { // |
                // left
                self.sub_room1 = Some(Rc::new(RefCell::new(Room::new(
                    self.x_bounds.start..split_point,
                    self.y_bounds.clone()
                ))));
                // right
                self.sub_room2 = Some(Rc::new(RefCell::new(Room::new(
                    split_point..self.x_bounds.end,
                    self.y_bounds.clone()
                ))));
                self.sub_room2.as_ref().unwrap().borrow_mut().left.push(self.sub_room1.clone().unwrap());
                self.sub_room1.as_ref().unwrap().borrow_mut().right.push(self.sub_room2.clone().unwrap());

            }
            Axis::Horizontal => { // -
                // down
                self.sub_room1 = Some(Rc::new(RefCell::new(Room::new(
                    self.x_bounds.clone(),
                    self.y_bounds.start..split_point
                ))));
                // up
                self.sub_room2 = Some(Rc::new(RefCell::new(Room::new(
                    self.x_bounds.clone(),
                    split_point..self.y_bounds.end
                ))));
                self.sub_room2.as_ref().unwrap().borrow_mut().down.push(self.sub_room1.clone().unwrap());
                self.sub_room1.as_ref().unwrap().borrow_mut().up.push(self.sub_room2.clone().unwrap());
            }
        }
        // If performance problems, change this:
        //std::thread::sleep(std::time::Duration::from_secs(1));
        self.sub_room1.as_mut().unwrap().borrow_mut().subdivide();
        self.sub_room2.as_mut().unwrap().borrow_mut().subdivide();
    }
    fn create_map_rooms(&self, board: &mut Board) {
        if self.sub_room1.is_none() {
            board.make_room(
                Vector::new(self.x_bounds.start, self.y_bounds.start),
                Vector::new(self.x_bounds.end+1, self.y_bounds.end+1)
            );
        }
        else {
            self.sub_room1.as_ref().unwrap().borrow().create_map_rooms(board);
            self.sub_room2.as_ref().unwrap().borrow().create_map_rooms(board);
        }
    }
    fn remove_extra_adjacents(&mut self) {
        if self.sub_room1.is_some() {
            self.sub_room1.as_ref().unwrap().borrow_mut().remove_extra_adjacents();
            self.sub_room2.as_ref().unwrap().borrow_mut().remove_extra_adjacents();
            return
        }
        self.up.retain(|other| {
            let bounds = other.borrow().x_bounds.clone();
            if bounds.start <= self.x_bounds.start && bounds.end <= self.x_bounds.start { false }
            else if bounds.end >= self.x_bounds.end && bounds.start >= self.x_bounds.end { false }
            else { true }
        });
        self.down.retain(|other| {
            let bounds = other.borrow().x_bounds.clone();
            if bounds.start <= self.x_bounds.start && bounds.end <= self.x_bounds.start { false }
            else if bounds.end >= self.x_bounds.end && bounds.start >= self.x_bounds.end { false }
            else { true }
        });
        self.left.retain(|other| {
            let bounds = other.borrow().y_bounds.clone();
            if bounds.start <= self.y_bounds.start && bounds.end <= self.y_bounds.start { false }
            else if bounds.end >= self.y_bounds.end && bounds.start >= self.y_bounds.end { false }
            else { true }
        });
        self.right.retain(|other| {
            let bounds = other.borrow().y_bounds.clone();
            if bounds.start <= self.y_bounds.start && bounds.end <= self.y_bounds.start { false }
            else if bounds.end >= self.y_bounds.end && bounds.start >= self.y_bounds.end { false }
            else { true }
        });
    }
    fn place_doors(&self, board: &mut Board) {
        if self.sub_room1.is_some() {
            self.sub_room1.as_ref().unwrap().borrow().place_doors(board);
            self.sub_room2.as_ref().unwrap().borrow().place_doors(board);
            return;
        }
        for up in self.up.iter() {
            let low = self.x_bounds.start.max(up.borrow().x_bounds.start);
            let high = self.x_bounds.end.min(up.borrow().x_bounds.end);
            debug!(assert!(low < high));
            board[
                Vector::new(low.midpoint(high), self.y_bounds.end)
            ] = Some(Piece::Door(Door { open: false }));
        }
        for down in self.down.iter() {
            let low = self.x_bounds.start.max(down.borrow().x_bounds.start);
            let high = self.x_bounds.end.min(down.borrow().x_bounds.end);
            debug!(assert!(low < high));
            board[
                Vector::new(low.midpoint(high), self.y_bounds.start)
            ] = Some(Piece::Door(Door { open: false }));
        }
        for left in self.left.iter() {
            let low = self.y_bounds.start.max(left.borrow().y_bounds.start);
            let high = self.y_bounds.end.min(left.borrow().y_bounds.end);
            debug!(assert!(low < high));
            board[
                Vector::new(self.x_bounds.start, low.midpoint(high))
            ] = Some(Piece::Door(Door { open: false }));
        }
        for right in self.right.iter() {
            let low = self.y_bounds.start.max(right.borrow().y_bounds.start);
            let high = self.y_bounds.end.min(right.borrow().y_bounds.end);
            debug!(assert!(low < high));
            board[
                Vector::new(self.x_bounds.start, low.midpoint(high))
            ] = Some(Piece::Door(Door { open: false }));
        }
    }
}
enum Axis {
    Horizontal,
    Vertical
}
impl std::ops::Not for Axis {
    type Output = Axis;
    fn not(self) -> Self::Output {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }
}
