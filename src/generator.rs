use crate::{
    Board, Enemy, MapGenSettings, Style, Vector, board::Piece, enemy::Variant, pieces::door::Door,
    random::*,
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
const STYLE: Style = *Style::new().green();
pub static DO_DELAY: AtomicBool = AtomicBool::new(false);

pub fn generate(settings: MapGenSettings) -> JoinHandle<Board> {
    std::thread::spawn(move || {
        crate::log!(
            "{}Generating map with settings: {:#?}\x1b[0m",
            STYLE,
            settings
        );
        let start = std::time::Instant::now();
        let mut room = Room::new(0..(settings.x - 1), 0..(settings.y - 1), settings.budget);
        room.subdivide();
        room.fill_leaf_adjacents(room.get_all_leafs().as_slice());
        room.remove_extra_adjacents(None);
        let mut board = Board::new(settings.x, settings.y, settings.render_x, settings.render_y);
        room.create_map_rooms(&mut board);
        room.place_doors(&mut board);
        crate::log!("{STYLE}Begining enemy placement\x1b[0m");
        room.place_enemies(&mut board);
        promote_boss(&mut board, settings.num_bosses);
        let elapsed = start.elapsed();
        crate::log!(
            "{STYLE}Map gen time: {}s({}ms)\x1b[0m",
            elapsed.as_secs(),
            elapsed.as_millis()
        );
        //checksum(&board);
        board
    })
}
fn promote_boss(board: &mut Board, num_bosses: usize) {
    let mut current_tier = board.get_highest_tier();
    let mut potential = Vec::new();
    for _ in 0..num_bosses {
        crate::log!("{STYLE}Attempting to create boss of tier: {current_tier}\x1b[0m");
        board.get_all_of_tier(current_tier, &mut potential);
        let mut failed = true;
        delay();
        if let Some(index) = random_index(potential.len()) {
            crate::log!(
                "{STYLE}Selected enemy, promoting from {}\x1b[0m",
                potential[index].try_read().unwrap().variant
            );
            board.bosses.push(crate::board::Boss {
                last_pos: Vector::new(0, 0),
                sibling: Arc::downgrade(&potential[index]),
            });
            potential[index].try_write().unwrap().promote().unwrap();
            failed = false;
        }
        delay();
        if current_tier > 1 && (failed || bool::random()) {
            // > 1 because lowest is 1
            crate::log!("{STYLE}Decrimenting current tier\x1b[0m");
            current_tier -= 1;
        }
        potential.truncate(0);
    }
}
fn attempt_pick_pos(
    board: &mut Board,
    x_range: &Range<usize>,
    y_range: &Range<usize>,
) -> Option<Vector> {
    for _ in 0..10 {
        delay();
        let x = random_in_usize_range(x_range);
        delay();
        let y = random_in_usize_range(y_range);
        let pos = Vector::new(x, y);
        if board.get_enemy(pos, None).is_none() && board[pos].is_none() {
            return Some(pos);
        }
    }
    None
}
fn checksum(board: &Board) {
    if board.bosses.len() == 0 {
        panic!("No bosses");
    }
    for enemy in board.enemies.iter() {
        let pos = enemy.try_read().unwrap().pos;
        let addr = Arc::as_ptr(enemy).addr();
        if let Some(piece) = &board[pos] {
            panic!("Enemy spawned inside: {piece}");
        }
        if let Some(_) = board.get_enemy(pos, Some(addr)) {
            panic!("Enemy spawned inside another enemy")
        }
    }
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
        let mut force_axis = None;
        if x_len <= MINIMUM || y_len <= MINIMUM {
            if max {
                if x_len <= MINIMUM {
                    force_axis = Some(Axis::Vertical)
                } else {
                    force_axis = Some(Axis::Horizontal)
                }
            } else {
                return;
            }
        }
        let mut axis;
        if x_len > y_len {
            axis = Axis::Vertical;
        } else {
            axis = Axis::Horizontal;
        }
        delay();
        if (max && force_axis.is_none()) && random() & 0b0011_1111 == 0 {
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
            let low = self.x_bounds.start.max(other.borrow().x_bounds.start);
            let high = self.x_bounds.end.min(other.borrow().x_bounds.end);
            if self.y_bounds.end != other.borrow().y_bounds.start {
                false
            } else if low >= high {
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
            let low = self.x_bounds.start.max(other.borrow().x_bounds.start);
            let high = self.x_bounds.end.min(other.borrow().x_bounds.end);
            if self.y_bounds.start != other.borrow().y_bounds.end {
                false
            } else if low >= high {
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
            let low = self.y_bounds.start.max(other.borrow().y_bounds.start);
            let high = self.y_bounds.end.min(other.borrow().y_bounds.end);
            if self.x_bounds.start != other.borrow().x_bounds.end {
                false
            } else if low >= high {
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
            let low = self.y_bounds.start.max(other.borrow().y_bounds.start);
            let high = self.y_bounds.end.min(other.borrow().y_bounds.end);
            if self.x_bounds.end != other.borrow().x_bounds.start {
                false
            } else if low >= high {
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
            board[Vector::new(self.x_bounds.end, low.midpoint(high))] =
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
        if self.budget == 0 {
            return;
        }
        crate::log!("{STYLE}Begining enemy placement for room\x1b[0m");
        let mut budget = self.budget;
        let (center_variant, center_num) = Variant::pick_variant(budget, true);
        // Placing centers
        debug_assert_ne!(center_num, 0, "Attempted to spawn 0 centers");
        crate::log!(
            "{STYLE}Attempting to place {center_num} centers of variant: {center_variant}\x1b[0m"
        );
        let mut centers = Vec::new();
        for _ in 0..center_num {
            if let Some(pos) = attempt_pick_pos(board, &self.x_bounds, &self.y_bounds) {
                centers.push(pos);
                budget -= center_variant.get_cost().unwrap();
                board.enemies.push(Arc::new(RwLock::new(Enemy::new(
                    pos,
                    center_variant.clone(),
                ))));
            }
        }
        if centers.len() == 0 {
            crate::log!("{STYLE}Failed to place centers\x1b[0m");
            return;
        }
        let budget_per_center = budget / centers.len();
        // Allocating budgets
        let mut centers: Vec<(Vector, usize)> = centers
            .into_iter()
            .map(|center| {
                budget -= budget_per_center;
                (center, budget_per_center)
            })
            .collect();
        if budget > 0 {
            centers.last_mut().unwrap().1 += budget;
        }
        // Placing surroundings
        for (center, mut budget) in centers.into_iter() {
            let mut available = board.flood_within(center, 3, true);
            let mut fails = 0;
            while budget > 0 {
                if fails > 10 {
                    crate::log!("{STYLE}Fails accumulated too much, stopping\x1b[0m");
                }
                delay();
                let pos = available.swap_remove(random_index(available.len()).unwrap());
                if board.get_enemy(pos, None).is_some() {
                    fails += 1;
                    continue;
                }
                fails = 0;
                let variant = Variant::pick_variant(budget, false).0;
                budget -= variant.get_cost().unwrap();
                board
                    .enemies
                    .push(Arc::new(RwLock::new(Enemy::new(pos, variant))));
            }
        }
        crate::log!("{STYLE}Done placing enemies for room\x1b[0m");
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
