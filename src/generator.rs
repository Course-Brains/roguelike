use crate::{
    Board, Enemy, Vector, board::Piece, enemy::Variant, pieces::door::Door, pieces::wall::Wall,
    random, random_in_range,
};
use albatrice::debug;
use std::cell::RefCell;
use std::ops::Range;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;

const INTERVAL: usize = 5;
const MINIMUM: usize = 10;
const MAXIMUM: usize = 100;
const DELAY: std::time::Duration = std::time::Duration::from_millis(100);
pub static DO_DELAY: AtomicBool = AtomicBool::new(false);

pub fn generate(
    x: usize,
    y: usize,
    render_x: usize,
    render_y: usize,
    budget: usize,
) -> JoinHandle<Board> {
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let mut room = Room::new(0..(x - 1), 0..(y - 1), budget);
        room.subdivide();
        room.fill_leaf_adjacents(room.get_all_leafs().as_slice());
        room.remove_extra_adjacents(None);
        let mut board = Board::new(x, y, render_x, render_y);
        room.create_map_rooms(&mut board);
        room.place_doors(&mut board);
        remove_edge_doors(&mut board);
        room.place_enemies(&mut board);
        let elapsed = start.elapsed();
        crate::log!(
            "Map gen time: {}s({}ms)",
            elapsed.as_secs(),
            elapsed.as_millis()
        );
        board
    })
}
fn remove_edge_doors(board: &mut Board) {
    let board_x = board.x;
    let board_y = board.y;
    for x in 0..(board_x - 1) {
        if let Some(Piece::Door(_)) = board[Vector::new(x, board_y - 1)] {
            board[Vector::new(x, board_y - 1)] = Some(Piece::Wall(Wall {}));
        }
        if let Some(Piece::Door(_)) = board[Vector::new(x, 0)] {
            board[Vector::new(x, 0)] = Some(Piece::Wall(Wall {}));
        }
    }
    for y in 0..(board_y - 1) {
        if let Some(Piece::Door(_)) = board[Vector::new(board_x - 1, y)] {
            board[Vector::new(board_x - 1, y)] = Some(Piece::Wall(Wall {}));
        }
        if let Some(Piece::Door(_)) = board[Vector::new(0, y)] {
            board[Vector::new(0, y)] = Some(Piece::Wall(Wall {}));
        }
    }
}
fn delay() {
    if DO_DELAY.load(Ordering::SeqCst) {
        std::thread::sleep(DELAY)
    }
}
type Adjacent = Vec<Arc<RwLock<Room>>>;
struct Room {
    x_bounds: Range<usize>,
    y_bounds: Range<usize>,
    sub_room1: Option<Arc<RwLock<Room>>>,
    sub_room2: Option<Arc<RwLock<Room>>>,
    up: RefCell<Adjacent>,
    down: RefCell<Adjacent>,
    left: RefCell<Adjacent>,
    right: RefCell<Adjacent>,
    budget: usize,
}
impl Room {
    fn new(x_bounds: Range<usize>, y_bounds: Range<usize>, budget: usize) -> Room {
        Room {
            x_bounds,
            y_bounds,
            sub_room1: None,
            sub_room2: None,
            up: RefCell::new(Vec::new()),
            down: RefCell::new(Vec::new()),
            left: RefCell::new(Vec::new()),
            right: RefCell::new(Vec::new()),
            budget,
        }
    }
    fn subdivide(&mut self) {
        let x_len = self.x_bounds.end - self.x_bounds.start;
        let y_len = self.y_bounds.end - self.y_bounds.start;
        let max = x_len > MAXIMUM || y_len > MAXIMUM;
        debug!({
            assert_eq!(x_len % INTERVAL, 0, "x_len = {x_len}");
            assert_eq!(y_len % INTERVAL, 0, "y_len = {y_len}");
        });
        if x_len <= MINIMUM || x_len <= MINIMUM {
            return;
        }
        let mut axis;
        if x_len > y_len {
            axis = Axis::Vertical;
        } else {
            axis = Axis::Horizontal;
        }
        delay();
        if max {
            if random() & 0b0011_1111 == 0 {
                // 1 in 64 to do the other axis instead
                axis = !axis;
            }
        }
        let axis_len = match axis {
            Axis::Vertical => x_len,
            Axis::Horizontal => y_len,
        };
        let axis_bounds = match axis {
            Axis::Vertical => &self.x_bounds,
            Axis::Horizontal => &self.y_bounds,
        };
        let num_splits = axis_len / INTERVAL - 2;
        if num_splits == 0 {
            return;
        }
        let mut split_point;
        loop {
            delay();
            split_point = (random() % num_splits as u8 + 1) as usize * INTERVAL + axis_bounds.start;
            debug!({
                assert!(split_point > axis_bounds.start);
                assert!(split_point < axis_bounds.end);
            });
            if split_point - axis_bounds.start < MINIMUM {
                if max {
                    continue;
                }
                return;
            }
            if axis_bounds.end - split_point < MINIMUM {
                if max {
                    continue;
                }
                return;
            }
            break;
        }
        let ratio = (split_point - axis_bounds.start) as f32 / axis_len as f32;
        let split_budget = (self.budget as f32 * ratio) as usize;
        match axis {
            Axis::Vertical => {
                // |
                // left
                self.sub_room2 = Some(Arc::new(RwLock::new(Room::new(
                    self.x_bounds.start..split_point,
                    self.y_bounds.clone(),
                    split_budget,
                ))));
                // right
                self.sub_room1 = Some(Arc::new(RwLock::new(Room::new(
                    split_point..self.x_bounds.end,
                    self.y_bounds.clone(),
                    self.budget - split_budget,
                ))));
            }
            Axis::Horizontal => {
                // -
                // down
                self.sub_room1 = Some(Arc::new(RwLock::new(Room::new(
                    self.x_bounds.clone(),
                    self.y_bounds.start..split_point,
                    split_budget,
                ))));
                // up
                self.sub_room2 = Some(Arc::new(RwLock::new(Room::new(
                    self.x_bounds.clone(),
                    split_point..self.y_bounds.end,
                    self.budget - split_budget,
                ))));
            }
        }
        // If performance problems, change this:
        //std::thread::sleep(std::time::Duration::from_secs(1));
        self.sub_room1
            .as_ref()
            .unwrap()
            .write()
            .unwrap()
            .subdivide();
        self.sub_room2
            .as_ref()
            .unwrap()
            .write()
            .unwrap()
            .subdivide();
    }
    fn create_map_rooms(&self, board: &mut Board) {
        if self.sub_room1.is_none() {
            board.make_room(
                Vector::new(self.x_bounds.start, self.y_bounds.start),
                Vector::new(self.x_bounds.end + 1, self.y_bounds.end + 1),
            );
        } else {
            self.sub_room1
                .as_ref()
                .unwrap()
                .write()
                .unwrap()
                .create_map_rooms(board);
            self.sub_room2
                .as_ref()
                .unwrap()
                .write()
                .unwrap()
                .create_map_rooms(board);
        }
    }
    fn fill_leaf_adjacents(&mut self, adj: &[Arc<RwLock<Room>>]) {
        if self.sub_room1.is_some() {
            self.sub_room1
                .as_ref()
                .unwrap()
                .write()
                .unwrap()
                .fill_leaf_adjacents(adj);
            self.sub_room2
                .as_ref()
                .unwrap()
                .write()
                .unwrap()
                .fill_leaf_adjacents(adj);
            return;
        }
        self.up = RefCell::new(adj.to_vec());
        self.down = RefCell::new(adj.to_vec());
        self.left = RefCell::new(adj.to_vec());
        self.right = RefCell::new(adj.to_vec());
    }
    fn get_all_leafs(&self) -> Vec<Arc<RwLock<Room>>> {
        let mut out = Vec::new();
        self.append_all_leafs(&mut out, None);
        out
    }
    fn append_all_leafs(&self, out: &mut Vec<Arc<RwLock<Room>>>, this: Option<Arc<RwLock<Room>>>) {
        if self.sub_room1.is_some() {
            self.sub_room1
                .as_ref()
                .unwrap()
                .read()
                .unwrap()
                .append_all_leafs(out, Some(self.sub_room1.clone().unwrap()));
            self.sub_room2
                .as_ref()
                .unwrap()
                .read()
                .unwrap()
                .append_all_leafs(out, Some(self.sub_room2.clone().unwrap()));
        } else {
            out.push(this.unwrap());
        }
    }
    fn remove_extra_adjacents(&mut self, addr: Option<usize>) {
        if self.sub_room1.is_some() {
            self.sub_room1
                .as_ref()
                .unwrap()
                .write()
                .unwrap()
                .remove_extra_adjacents(Some(Arc::as_ptr(self.sub_room1.as_ref().unwrap()).addr()));
            self.sub_room2
                .as_ref()
                .unwrap()
                .write()
                .unwrap()
                .remove_extra_adjacents(Some(Arc::as_ptr(self.sub_room2.as_ref().unwrap()).addr()));
            return;
        }
        assert!(addr.is_some());
        self.up.borrow_mut().retain(|other| {
            if Arc::as_ptr(other).addr() == addr.unwrap() {
                return false;
            }
            let bounds = other.read().unwrap().x_bounds.clone();
            if self.y_bounds.end != other.read().unwrap().y_bounds.start {
                false
            } else if bounds.start <= self.x_bounds.start && bounds.end <= self.x_bounds.start {
                false
            } else if bounds.end >= self.x_bounds.end && bounds.start >= self.x_bounds.end {
                false
            } else {
                true
            }
        });
        self.down.borrow_mut().retain(|other| {
            if Arc::as_ptr(other).addr() == addr.unwrap() {
                return false;
            }
            let bounds = other.read().unwrap().x_bounds.clone();
            if self.y_bounds.start != other.read().unwrap().y_bounds.end {
                false
            } else if bounds.start <= self.x_bounds.start && bounds.end <= self.x_bounds.start {
                false
            } else if bounds.end >= self.x_bounds.end && bounds.start >= self.x_bounds.end {
                false
            } else {
                true
            }
        });
        self.left.borrow_mut().retain(|other| {
            if Arc::as_ptr(other).addr() == addr.unwrap() {
                return false;
            }
            let bounds = other.read().unwrap().y_bounds.clone();
            if self.x_bounds.start != other.read().unwrap().x_bounds.end {
                false
            } else if bounds.start <= self.y_bounds.start && bounds.end <= self.y_bounds.start {
                false
            } else if bounds.end >= self.y_bounds.end && bounds.start >= self.y_bounds.end {
                false
            } else {
                true
            }
        });
        self.right.borrow_mut().retain(|other| {
            if Arc::as_ptr(other).addr() == addr.unwrap() {
                return false;
            }
            let bounds = other.read().unwrap().y_bounds.clone();
            if self.x_bounds == bounds && self.y_bounds == other.read().unwrap().y_bounds {
                false
            } else if self.x_bounds.end != other.read().unwrap().x_bounds.start {
                false
            } else if bounds.start <= self.y_bounds.start && bounds.end <= self.y_bounds.start {
                false
            } else if bounds.end >= self.y_bounds.end && bounds.start >= self.y_bounds.end {
                false
            } else {
                true
            }
        });
    }
    fn place_doors(&self, board: &mut Board) {
        if self.sub_room1.is_some() {
            self.sub_room1
                .as_ref()
                .unwrap()
                .read()
                .unwrap()
                .place_doors(board);
            self.sub_room2
                .as_ref()
                .unwrap()
                .read()
                .unwrap()
                .place_doors(board);
            return;
        }
        for up in self.up.borrow().iter() {
            let low = self.x_bounds.start.max(up.read().unwrap().x_bounds.start);
            let high = self.x_bounds.end.min(up.read().unwrap().x_bounds.end);
            debug!(assert!(low < high));
            board[Vector::new(low.midpoint(high), self.y_bounds.end)] =
                Some(Piece::Door(Door { open: false }));
        }
        for down in self.down.borrow().iter() {
            let low = self.x_bounds.start.max(down.read().unwrap().x_bounds.start);
            let high = self.x_bounds.end.min(down.read().unwrap().x_bounds.end);
            debug!(assert!(low < high));
            board[Vector::new(low.midpoint(high), self.y_bounds.start)] =
                Some(Piece::Door(Door { open: false }));
        }
        for left in self.left.borrow().iter() {
            let low = self.y_bounds.start.max(left.read().unwrap().y_bounds.start);
            let high = self.y_bounds.end.min(left.read().unwrap().y_bounds.end);
            debug!(assert!(low < high));
            board[Vector::new(self.x_bounds.start, low.midpoint(high))] =
                Some(Piece::Door(Door { open: false }));
        }
        for right in self.right.borrow().iter() {
            let low = self
                .y_bounds
                .start
                .max(right.read().unwrap().y_bounds.start);
            let high = self.y_bounds.end.min(right.read().unwrap().y_bounds.end);
            debug!(assert!(low < high));
            board[Vector::new(self.x_bounds.start, low.midpoint(high))] =
                Some(Piece::Door(Door { open: false }));
        }
    }
    fn place_enemies(&self, board: &mut Board) {
        if self.sub_room1.is_some() {
            self.sub_room1
                .as_ref()
                .unwrap()
                .read()
                .unwrap()
                .place_enemies(board);
            self.sub_room2
                .as_ref()
                .unwrap()
                .read()
                .unwrap()
                .place_enemies(board);
            return;
        }
        let mut budget = self.budget;
        'outer: while budget > 0 {
            delay();
            let pos = Vector::new(
                random_in_range(0..(self.x_bounds.end - self.x_bounds.start - 2) as u8) as usize
                    + self.x_bounds.start
                    + 1,
                random_in_range(0..(self.y_bounds.end - self.y_bounds.start - 2) as u8) as usize
                    + self.y_bounds.start
                    + 1,
            );
            for enemy in board.enemies.iter() {
                if enemy.read().unwrap().pos == pos {
                    continue 'outer;
                }
            }
            let variant = {
                if budget >= 5 {
                    budget -= 5;
                    Variant::Mage(crate::enemy::Spell::Teleport)
                } else {
                    budget -= 1;
                    Variant::Basic
                }
            };
            board
                .enemies
                .push(Arc::new(RwLock::new(Enemy::new(pos, variant))));
        }
    }
}
enum Axis {
    Horizontal,
    Vertical,
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
