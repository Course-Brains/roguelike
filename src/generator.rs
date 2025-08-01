use crate::{
    Board, Enemy, MapGenSettings, Vector, board::Piece, enemy::Variant, pieces::door::Door,
    pieces::wall::Wall, random, random_in_range,
};
use albatrice::debug;
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
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

pub fn generate(settings: MapGenSettings) -> JoinHandle<Board> {
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let mut room = Room::new(0..(settings.x - 1), 0..(settings.y - 1), settings.budget);
        room.subdivide();
        room.fill_leaf_adjacents(room.get_all_leafs().as_slice());
        room.remove_extra_adjacents(None);
        let mut board = Board::new(settings.x, settings.y, settings.render_x, settings.render_y);
        room.create_map_rooms(&mut board);
        room.place_doors(&mut board);
        remove_edge_doors(&mut board);
        room.place_enemies(&mut board);
        promote_boss(&mut board);
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
fn promote_boss(board: &mut Board) {
    let mut highest = 1;
    for enemy in board.enemies.iter() {
        if enemy.try_read().unwrap().variant.get_tier().unwrap() > highest {
            highest = enemy.try_read().unwrap().variant.get_tier().unwrap();
        }
    }
    crate::log!("highest enemy is tier {highest}");
    let mut candidates = Vec::new();
    for enemy in board.enemies.iter() {
        if enemy.try_read().unwrap().variant.get_tier().unwrap() == highest {
            candidates.push(enemy.clone());
        }
    }
    let boss = match candidates.len() > 256 {
        true => candidates.swap_remove(random() as usize),
        false => candidates.swap_remove(random() as usize % candidates.len()),
    };
    crate::log!(
        "boss will be a {} at {}",
        boss.try_read().unwrap().variant,
        boss.try_read().unwrap().pos
    );
    board.boss = Some(Arc::downgrade(&boss));
    board.boss_pos = boss.try_read().unwrap().pos;
    boss.try_write().unwrap().promote().unwrap()
}
fn delay() {
    if DO_DELAY.load(Ordering::SeqCst) {
        std::thread::sleep(DELAY)
    }
}
type Adjacent = Vec<Rc<RefCell<Room>>>;
struct Room {
    x_bounds: Range<usize>,
    y_bounds: Range<usize>,
    sub_room1: Option<Rc<RefCell<Room>>>,
    sub_room2: Option<Rc<RefCell<Room>>>,
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
        if x_len <= MINIMUM || y_len <= MINIMUM {
            return;
        }
        let mut axis;
        if x_len > y_len {
            axis = Axis::Vertical;
        } else {
            axis = Axis::Horizontal;
        }
        delay();
        if max && random() & 0b0011_1111 == 0 {
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
        let num_splits = axis_len / INTERVAL - 2;
        if num_splits == 0 {
            return;
        }
        let mut split_point = None;
        let mut fails = 0;
        while fails < 10 {
            delay();
            let split = (random() % num_splits as u8 + 1) as usize * INTERVAL + axis_bounds.start;
            debug!({
                assert!(split > axis_bounds.start);
                assert!(split < axis_bounds.end);
            });
            if split - axis_bounds.start < MINIMUM {
                if max {
                    fails += 1;
                    continue;
                }
                return;
            }
            if axis_bounds.end - split < MINIMUM {
                if max {
                    fails += 1;
                    continue;
                }
                return;
            }
            split_point = Some(split);
            break;
        }
        if split_point.is_none() {
            return;
        }
        let split_point = split_point.unwrap();
        let ratio = (split_point - axis_bounds.start) as f32 / axis_len as f32;
        let split_budget = (self.budget as f32 * ratio) as usize;
        match axis {
            Axis::Vertical => {
                // |
                // left
                self.sub_room2 = Some(Rc::new(RefCell::new(Room::new(
                    self.x_bounds.start..split_point,
                    self.y_bounds.clone(),
                    split_budget,
                ))));
                // right
                self.sub_room1 = Some(Rc::new(RefCell::new(Room::new(
                    split_point..self.x_bounds.end,
                    self.y_bounds.clone(),
                    self.budget - split_budget,
                ))));
            }
            Axis::Horizontal => {
                // -
                // down
                self.sub_room1 = Some(Rc::new(RefCell::new(Room::new(
                    self.x_bounds.clone(),
                    self.y_bounds.start..split_point,
                    split_budget,
                ))));
                // up
                self.sub_room2 = Some(Rc::new(RefCell::new(Room::new(
                    self.x_bounds.clone(),
                    split_point..self.y_bounds.end,
                    self.budget - split_budget,
                ))));
            }
        }
        self.sub_room1.as_ref().unwrap().borrow_mut().subdivide();
        self.sub_room2.as_ref().unwrap().borrow_mut().subdivide();
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
                .borrow_mut()
                .create_map_rooms(board);
            self.sub_room2
                .as_ref()
                .unwrap()
                .borrow_mut()
                .create_map_rooms(board);
        }
    }
    fn fill_leaf_adjacents(&mut self, adj: &[Rc<RefCell<Room>>]) {
        if self.sub_room1.is_some() {
            self.sub_room1
                .as_ref()
                .unwrap()
                .borrow_mut()
                .fill_leaf_adjacents(adj);
            self.sub_room2
                .as_ref()
                .unwrap()
                .borrow_mut()
                .fill_leaf_adjacents(adj);
            return;
        }
        self.up = RefCell::new(adj.to_vec());
        self.down = RefCell::new(adj.to_vec());
        self.left = RefCell::new(adj.to_vec());
        self.right = RefCell::new(adj.to_vec());
    }
    fn get_all_leafs(&self) -> Vec<Rc<RefCell<Room>>> {
        let mut out = Vec::new();
        self.append_all_leafs(&mut out, None);
        out
    }
    fn append_all_leafs(&self, out: &mut Vec<Rc<RefCell<Room>>>, this: Option<Rc<RefCell<Room>>>) {
        if self.sub_room1.is_some() {
            self.sub_room1
                .as_ref()
                .unwrap()
                .borrow()
                .append_all_leafs(out, Some(self.sub_room1.clone().unwrap()));
            self.sub_room2
                .as_ref()
                .unwrap()
                .borrow()
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
                .borrow_mut()
                .remove_extra_adjacents(Some(Rc::as_ptr(self.sub_room1.as_ref().unwrap()).addr()));
            self.sub_room2
                .as_ref()
                .unwrap()
                .borrow_mut()
                .remove_extra_adjacents(Some(Rc::as_ptr(self.sub_room2.as_ref().unwrap()).addr()));
            return;
        }
        assert!(addr.is_some());
        self.up.borrow_mut().retain(|other| {
            if Rc::as_ptr(other).addr() == addr.unwrap() {
                return false;
            }
            let bounds = other.borrow().x_bounds.clone();
            if self.y_bounds.end != other.borrow().y_bounds.start {
                false
            } else {
                bounds.start < self.x_bounds.end
            }
        });
        self.down.borrow_mut().retain(|other| {
            if Rc::as_ptr(other).addr() == addr.unwrap() {
                return false;
            }
            let bounds = other.borrow().x_bounds.clone();
            if self.y_bounds.start != other.borrow().y_bounds.end {
                false
            } else {
                bounds.start < self.x_bounds.end
            }
        });
        self.left.borrow_mut().retain(|other| {
            if Rc::as_ptr(other).addr() == addr.unwrap() {
                return false;
            }
            let bounds = other.borrow().y_bounds.clone();
            if self.x_bounds.start != other.borrow().x_bounds.end {
                false
            } else {
                bounds.start < self.y_bounds.end
            }
        });
        self.right.borrow_mut().retain(|other| {
            if Rc::as_ptr(other).addr() == addr.unwrap() {
                return false;
            }
            let bounds = other.borrow().y_bounds.clone();
            if self.x_bounds.end != other.borrow().x_bounds.start {
                false
            } else {
                bounds.start < self.y_bounds.end
            }
        });
    }
    fn place_doors(&self, board: &mut Board) {
        if self.sub_room1.is_some() {
            self.sub_room1.as_ref().unwrap().borrow().place_doors(board);
            self.sub_room2.as_ref().unwrap().borrow().place_doors(board);
            return;
        }
        for up in self.up.borrow().iter() {
            let low = self.x_bounds.start.max(up.borrow().x_bounds.start);
            let high = self.x_bounds.end.min(up.borrow().x_bounds.end);
            debug!(assert!(low < high));
            board[Vector::new(low.midpoint(high), self.y_bounds.end)] =
                Some(Piece::Door(Door { open: false }));
        }
        for down in self.down.borrow().iter() {
            let low = self.x_bounds.start.max(down.borrow().x_bounds.start);
            let high = self.x_bounds.end.min(down.borrow().x_bounds.end);
            debug!(assert!(low < high));
            board[Vector::new(low.midpoint(high), self.y_bounds.start)] =
                Some(Piece::Door(Door { open: false }));
        }
        for left in self.left.borrow().iter() {
            let low = self.y_bounds.start.max(left.borrow().y_bounds.start);
            let high = self.y_bounds.end.min(left.borrow().y_bounds.end);
            debug!(assert!(low < high));
            board[Vector::new(self.x_bounds.start, low.midpoint(high))] =
                Some(Piece::Door(Door { open: false }));
        }
        for right in self.right.borrow().iter() {
            let low = self.y_bounds.start.max(right.borrow().y_bounds.start);
            let high = self.y_bounds.end.min(right.borrow().y_bounds.end);
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
                .borrow()
                .place_enemies(board);
            self.sub_room2
                .as_ref()
                .unwrap()
                .borrow()
                .place_enemies(board);
            return;
        }
        let mut budget = self.budget;
        let mut fails = 0;
        'outer: while budget > 0 && fails < 10 {
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
                if enemy.try_read().unwrap().pos == pos {
                    fails += 1;
                    continue 'outer;
                }
            }
            let variant = {
                if budget >= 5 {
                    budget -= 5;
                    Variant::mage()
                } else {
                    budget -= 1;
                    Variant::basic()
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
