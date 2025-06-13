use std::ops::Range;
use std::thread::JoinHandle;
use std::rc::Rc;
use std::cell::UnsafeCell;
use crate::{Board, random, Vector, pieces::door::Door, board::Piece, pieces::wall::Wall};
use albatrice::debug;
const INTERVAL: usize = 5;
const MINIMUM: usize = 10;
pub fn generate(x: usize, y: usize, render_x: usize, render_y: usize) -> JoinHandle<Board> {
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let mut room = Room::new(0..(x-1), 0..(y-1));
        room.subdivide();
        room.fill_leaf_adjacents(&room.get_all_leafs());
        room.remove_extra_adjacents();
        let mut board = Board::new(x, y, render_x, render_y);
        room.create_map_rooms(&mut board);
        room.place_doors(&mut board);
        remove_edge_doors(&mut board);
        let elapsed = start.elapsed();
        crate::log!("Map gen time: {}({})", elapsed.as_millis(), elapsed.as_nanos());
        board
    })
}
fn remove_edge_doors(board: &mut Board) {
    let board_x = board.x;
    let board_y = board.y;
    for x in 0..(board_x-1) {
        if let Some(Piece::Door(_)) = board[Vector::new(x, board_y-1)] {
            board[Vector::new(x, board_y-1)] = Some(Piece::Wall(Wall {}));
        }
        if let Some(Piece::Door(_)) = board[Vector::new(x, 0)] {
            board[Vector::new(x, 0)] = Some(Piece::Wall(Wall {}));
        }
    }
    for y in 0..(board_y-1) {
        if let Some(Piece::Door(_)) = board[Vector::new(board_x-1, y)] {
            board[Vector::new(board_x-1, y)] = Some(Piece::Wall(Wall {}));
        }
        if let Some(Piece::Door(_)) = board[Vector::new(0, y)] {
            board[Vector::new(0, y)] = Some(Piece::Wall(Wall {}));
        }
    }
}
type Adjacent = Vec<Rc<UnsafeCell<Room>>>;
struct Room {
    x_bounds: Range<usize>,
    y_bounds: Range<usize>,
    sub_room1: Option<Rc<UnsafeCell<Room>>>,
    sub_room2: Option<Rc<UnsafeCell<Room>>>,
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
                self.sub_room1 = Some(Rc::new(UnsafeCell::new(Room::new(
                    self.x_bounds.start..split_point,
                    self.y_bounds.clone()
                ))));
                // right
                self.sub_room2 = Some(Rc::new(UnsafeCell::new(Room::new(
                    split_point..self.x_bounds.end,
                    self.y_bounds.clone()
                ))));
            }
            Axis::Horizontal => { // -
                // down
                self.sub_room1 = Some(Rc::new(UnsafeCell::new(Room::new(
                    self.x_bounds.clone(),
                    self.y_bounds.start..split_point
                ))));
                // up
                self.sub_room2 = Some(Rc::new(UnsafeCell::new(Room::new(
                    self.x_bounds.clone(),
                    split_point..self.y_bounds.end
                ))));
            }
        }
        // If performance problems, change this:
        //std::thread::sleep(std::time::Duration::from_secs(1));
        unsafe {
            self.sub_room1.as_ref().unwrap().get().as_mut().unwrap().subdivide();
            self.sub_room2.as_ref().unwrap().get().as_mut().unwrap().subdivide();
        }
    }
    fn create_map_rooms(&self, board: &mut Board) {
        if self.sub_room1.is_none() {
            board.make_room(
                Vector::new(self.x_bounds.start, self.y_bounds.start),
                Vector::new(self.x_bounds.end+1, self.y_bounds.end+1)
            );
        }
        else {
            unsafe {
                self.sub_room1.as_ref().unwrap().get().as_ref().unwrap().create_map_rooms(board);
                self.sub_room2.as_ref().unwrap().get().as_ref().unwrap().create_map_rooms(board);
            }
        }
    }
    fn fill_leaf_adjacents(&mut self, adj: &Vec<Rc<UnsafeCell<Room>>>) {
        if self.sub_room1.is_some() {
            unsafe {
                self.sub_room1.as_ref().unwrap().get().as_mut().unwrap().fill_leaf_adjacents(adj);
                self.sub_room2.as_ref().unwrap().get().as_mut().unwrap().fill_leaf_adjacents(adj);
            }
            return
        }
        self.up = adj.clone();
        self.down = adj.clone();
        self.left = adj.clone();
        self.right = adj.clone();
    }
    fn get_all_leafs(&self) -> Vec<Rc<UnsafeCell<Room>>> {
        let mut out = Vec::new();
        self.append_all_leafs(&mut out, None);
        out
    }
    fn append_all_leafs(&self, out: &mut Vec<Rc<UnsafeCell<Room>>>, this: Option<Rc<UnsafeCell<Room>>>) {
        if self.sub_room1.is_some() {
            unsafe {
                self.sub_room1.as_ref().unwrap().get().as_ref().unwrap().append_all_leafs(out, Some(self.sub_room1.clone().unwrap()));
                self.sub_room2.as_ref().unwrap().get().as_ref().unwrap().append_all_leafs(out, Some(self.sub_room2.clone().unwrap()));
            }
        }
        else {
            out.push(this.unwrap());
        }

    }
    fn remove_extra_adjacents(&mut self) {
        if self.sub_room1.is_some() {
            unsafe {
                self.sub_room1.as_ref().unwrap().get().as_mut().unwrap().remove_extra_adjacents();
                self.sub_room2.as_ref().unwrap().get().as_mut().unwrap().remove_extra_adjacents();
            }
            return
        }
        self.up.retain(|other| {
            unsafe {
                let bounds = other.get().as_ref().unwrap().x_bounds.clone();
                if self.x_bounds == bounds && self.y_bounds == other.get().as_ref().unwrap().y_bounds { false }
                else if self.y_bounds.end != other.get().as_ref().unwrap().y_bounds.start { false }
                else if bounds.start <= self.x_bounds.start && bounds.end <= self.x_bounds.start { false }
                else if bounds.end >= self.x_bounds.end && bounds.start >= self.x_bounds.end { false }
                else { true }
            }
        });
        self.down.retain(|other| {
            unsafe {
                let bounds = other.get().as_ref().unwrap().x_bounds.clone();
                if self.x_bounds == bounds && self.y_bounds == other.get().as_ref().unwrap().y_bounds { false }
                else if self.y_bounds.start != other.get().as_ref().unwrap().y_bounds.end { false }
                else if bounds.start <= self.x_bounds.start && bounds.end <= self.x_bounds.start { false }
                else if bounds.end >= self.x_bounds.end && bounds.start >= self.x_bounds.end { false }
                else { true }
            }
        });
        self.left.retain(|other| {
            unsafe {
                let bounds = other.get().as_ref().unwrap().y_bounds.clone();
                if self.x_bounds == bounds && self.y_bounds == other.get().as_ref().unwrap().y_bounds { false }
                else if self.x_bounds.start != other.get().as_ref().unwrap().x_bounds.end { false }
                else if bounds.start <= self.y_bounds.start && bounds.end <= self.y_bounds.start { false }
                else if bounds.end >= self.y_bounds.end && bounds.start >= self.y_bounds.end { false }
                else { true }
            }
        });
        self.right.retain(|other| {
            unsafe {
                let bounds = other.get().as_ref().unwrap().y_bounds.clone();
                if self.x_bounds == bounds && self.y_bounds == other.get().as_ref().unwrap().y_bounds { false }
                else if self.x_bounds.end != other.get().as_ref().unwrap().x_bounds.start { false }
                else if bounds.start <= self.y_bounds.start && bounds.end <= self.y_bounds.start { false }
                else if bounds.end >= self.y_bounds.end && bounds.start >= self.y_bounds.end { false }
                else { true }
            }
        });
    }
    fn place_doors(&self, board: &mut Board) {
        unsafe {
            if self.sub_room1.is_some() {
                self.sub_room1.as_ref().unwrap().get().as_ref().unwrap().place_doors(board);
                self.sub_room2.as_ref().unwrap().get().as_ref().unwrap().place_doors(board);
                return;
            }
            for up in self.up.iter() {
                let low = self.x_bounds.start.max(up.get().as_ref().unwrap().x_bounds.start);
                let high = self.x_bounds.end.min(up.get().as_ref().unwrap().x_bounds.end);
                debug!(assert!(low < high));
                board[
                    Vector::new(low.midpoint(high), self.y_bounds.end)
                ] = Some(Piece::Door(Door { open: false }));
            }
            for down in self.down.iter() {
                let low = self.x_bounds.start.max(down.get().as_ref().unwrap().x_bounds.start);
                let high = self.x_bounds.end.min(down.get().as_ref().unwrap().x_bounds.end);
                debug!(assert!(low < high));
                board[
                    Vector::new(low.midpoint(high), self.y_bounds.start)
                ] = Some(Piece::Door(Door { open: false }));
            }
            for left in self.left.iter() {
                let low = self.y_bounds.start.max(left.get().as_ref().unwrap().y_bounds.start);
                let high = self.y_bounds.end.min(left.get().as_ref().unwrap().y_bounds.end);
                debug!(assert!(low < high));
                board[
                    Vector::new(self.x_bounds.start, low.midpoint(high))
                ] = Some(Piece::Door(Door { open: false }));
            }
            for right in self.right.iter() {
                let low = self.y_bounds.start.max(right.get().as_ref().unwrap().y_bounds.start);
                let high = self.y_bounds.end.min(right.get().as_ref().unwrap().y_bounds.end);
                debug!(assert!(low < high));
                board[
                    Vector::new(self.x_bounds.start, low.midpoint(high))
                ] = Some(Piece::Door(Door { open: false }));
            }
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
