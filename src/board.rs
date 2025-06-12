use crate::{Vector,
    Style,
    pieces::{wall::Wall, door::Door},
    input::Direction,
    Enemy,
    Random,
};
use std::io::Write;
use std::collections::{BinaryHeap, HashSet, HashMap, VecDeque};
use std::ops::Range;
pub struct Board {
    pub x: usize,
    pub y: usize,
    pub render_x: usize, // the offset from the player to the edge of the screen(think radius)
    pub render_y: usize,
    pub inner: Vec<Option<Piece>>,
    pub backtraces: Vec<BackTrace>,
    pub enemies: Vec<Enemy>
}
impl Board {
    pub fn new(x: usize, y: usize, render_x: usize, render_y: usize) -> Board {
        let mut inner = Vec::with_capacity(x*y);
        inner.resize_with(x*y, || None);
        let backtraces = vec![BackTrace::new(); x*y];
        Board {
            x,
            y,
            render_x,
            render_y,
            inner,
            enemies: Vec::new(),
            backtraces
        }
    }
    // returns whether or not the cursor has a background behind it
    pub fn render(&self, base: Vector) {
        let mut lock = std::collections::VecDeque::new();
        let x_bound = base.x..base.x+(self.render_x*2);
        let y_bound = base.y..base.y+(self.render_y*2);
        for y in y_bound.clone() {
            crossterm::queue!(lock,
                crossterm::cursor::MoveTo(0, (y-y_bound.start) as u16)
            ).unwrap();
            write!(lock, "\x1b[2K").unwrap();
            for x in x_bound.clone() {
                if let Some(piece) = &self[Vector::new(x, y)] {
                    let (ch, style) = piece.render(Vector::new(x,y), self);
                    crossterm::queue!(lock,
                        crossterm::cursor::MoveTo(
                            (x-x_bound.start) as u16,
                            (y-y_bound.start) as u16
                        )
                    ).unwrap();
                    if let Some(style) = style {
                        lock.write_fmt(format_args!("{}{}\x1b[0m", style.enact(), ch)).unwrap()
                    }
                    else {
                        lock.write_fmt(format_args!("{}", ch)).unwrap();
                    }
                }
            }
        }
        write!(lock, "\x1b[B\x1b[2K").unwrap();
        self.draw_enemies(&mut lock, x_bound, y_bound);
        // It may seem inefficient to have an intermediary buffer when stdout already
        // has one, but without this, there is a vsync type visual artifact
        std::io::stdout().write_all(lock.make_contiguous()).unwrap();
    }
    fn draw_enemies(&self, lock: &mut impl Write, x_bound: Range<usize>, y_bound: Range<usize>) {
        for enemy in self.enemies.iter() {
            if !x_bound.contains(&enemy.pos.x) { continue }
            if !y_bound.contains(&enemy.pos.y) { continue }
            crossterm::queue!(lock,
                crossterm::cursor::MoveTo(
                    (enemy.pos.x-x_bound.start) as u16,
                    (enemy.pos.y-y_bound.start) as u16
                )
            ).unwrap();
            match enemy.render() {
                (ch, Some(style)) => write!(lock, "{}{ch}\x1b[0m", style.enact()).unwrap(),
                (ch, None) => write!(lock, "{ch}").unwrap()
            }
        }
    }
    pub fn has_background(&self, pos: Vector) -> bool {
        if let Some(piece) = self[pos] {
            if piece.render(pos, self).1.is_some_and(|x| x.has_background() ) { return true }
        }
        for enemy in self.enemies.iter() {
            if enemy.pos == pos {
                if let Some(style) = enemy.render().1 {
                    if style.has_background() { return true }
                }
                break
            }
        }
        false
    }
    pub fn generate_nav_data(&mut self, player: Vector) {
        if self.enemies.len() == 0 { return }
        let start = std::time::Instant::now();
        for item in self.backtraces.iter_mut() {
            item.cost = None;
        }
        let elapsed = start.elapsed();
        crate::log!("clear path time: {}({})", elapsed.as_millis(), elapsed.as_nanos());
        let start = std::time::Instant::now();
        let mut to_visit: BinaryHeap<PathData> = BinaryHeap::new();
        let mut visited = HashSet::new();
        to_visit.push(PathData::new(
            Direction::Up,
            player,
            self.enemies[0].pos,
            0
        ));
        for enemy in self.enemies.iter() {
            if !enemy.reachable { continue }
            if self.backtraces[self.to_index(enemy.pos)].cost.is_some() { continue }
            to_visit = to_visit.iter().map(|item| 
                PathData::new(item.from, item.pos, enemy.pos, item.cost)
            ).collect();
            
            while let Some(path_data) = to_visit.pop() {
                let index = self.to_index(path_data.pos);
                if self.backtraces[index].cost.is_none_or(|cost| cost > path_data.cost) {
                    self.backtraces[index].cost = Some(path_data.cost);
                    self.backtraces[index].from = path_data.from;
                }
                if path_data.pos == enemy.pos { break }
                if visited.contains(&path_data.pos) { continue }
                visited.insert(path_data.pos);
                let adj = self.get_adjacent(path_data.pos, Some(player));
                if adj.up {
                    to_visit.push(PathData::new(
                        Direction::Down,
                        path_data.pos+Direction::Up,
                        enemy.pos,
                        path_data.cost+1
                    ));
                }
                if adj.down {
                    to_visit.push(PathData::new(
                        Direction::Up,
                        path_data.pos+Direction::Down,
                        enemy.pos,
                        path_data.cost+1
                    ))
                }
                if adj.left {
                    to_visit.push(PathData::new(
                        Direction::Right,
                        path_data.pos+Direction::Left,
                        enemy.pos,
                        path_data.cost+1
                    ))
                }
                if adj.right {
                    to_visit.push(PathData::new(
                        Direction::Left,
                        path_data.pos+Direction::Right,
                        enemy.pos,
                        path_data.cost+1
                    ))
                }
            }
        }
        let elapsed = start.elapsed();
        crate::log!("path calc time: {}({})", elapsed.as_millis(), elapsed.as_nanos());
    }
    fn get_adjacent(&self, pos: Vector, player: Option<Vector>) -> Adj {
        let mut out = Adj::new(true);

        if pos.y == 0 { out.up = false }
        else if let Some(piece) = self[pos+Direction::Up] {
           if piece.has_collision() { out.up = false }
        }
        else if let Some(player) = player {
            if player.x.abs_diff((pos+Direction::Up).x) < 2 {
                if player.y.abs_diff((pos+Direction::Up).y) < 2 {
                    if self.contains_enemy(pos+Direction::Up) { out.up = false }
                }
            }
        }

        if pos.y >= self.y-1 { out.down = false }
        else if let Some(piece) = self[pos+Direction::Down] {
            if piece.has_collision() { out.down = false }
        }
        else if let Some(player) = player {
            if player.x.abs_diff((pos+Direction::Down).x) < 2 {
                if player.y.abs_diff((pos+Direction::Down).y) < 2 {
                    if self.contains_enemy(pos+Direction::Down) { out.down = false }
                }
            }
        }

        if pos.x == 0 { out.left = false }
        else if let Some(piece) = self[pos+Direction::Left] {
            if piece.has_collision() { out.left = false }
        }
        else if let Some(player) = player {
            if player.x.abs_diff((pos+Direction::Left).x) < 2 {
                if player.y.abs_diff((pos+Direction::Left).y) < 2 {
                    if self.contains_enemy(pos+Direction::Left) { out.left = false }
                }
            }
        }

        if pos.x >= self.x-1 { out.right = false }
        else if let Some(piece) = self[pos+Direction::Right] {
            if piece.has_collision() { out.right = false }
        }
        else if let Some(player) = player {
            if player.x.abs_diff((pos+Direction::Right).x) < 2 {
                if player.y.abs_diff((pos+Direction::Right).y) < 2 {
                    if self.contains_enemy(pos+Direction::Right) { out.right = false }
                }
            }
        }

        out
    }
    pub fn flood(&mut self, player: Vector) {
        let start = std::time::Instant::now();
        let mut lookup = HashMap::new();
        for (index, enemy) in self.enemies.iter_mut().enumerate() {
            enemy.reachable = false;
            lookup.insert(enemy.pos, index);
        }
        let mut to_visit = VecDeque::new();
        let mut seen = HashSet::new();
        to_visit.push_front(player);
        seen.insert(player);
        while let Some(pos) = to_visit.pop_back() {
            if let Some(index) = lookup.get(&pos) {
                self.enemies[*index].reachable = true;
            }
            let adj = self.get_adjacent(pos, None);
            if adj.up {
                if pos.y == 0 { crate::log!("up at {pos}") }
                if !seen.contains(&(pos+Direction::Up)) {
                    to_visit.push_front(pos+Direction::Up);
                    seen.insert(pos+Direction::Up);
                }
            }
            if adj.down {
                if pos.y >= self.y-1 { crate::log!("down at {pos}") }
                if !seen.contains(&(pos+Direction::Down)) {
                    to_visit.push_front(pos+Direction::Down);
                    seen.insert(pos+Direction::Down);
                }
            }
            if adj.left {
                if pos.x == 0 { crate::log!("left at {pos}") }
                if !seen.contains(&(pos+Direction::Left)) {
                    to_visit.push_front(pos+Direction::Left);
                    seen.insert(pos+Direction::Left);
                }
            }
            if adj.right {
                if pos.x >= self.x-1 { crate::log!("right at {pos}") }
                if !seen.contains(&(pos+Direction::Right)) {
                    to_visit.push_front(pos+Direction::Right);
                    seen.insert(pos+Direction::Right);
                }
            }
        }
        let elapsed = start.elapsed();
        crate::log!("flood time: {}({})", elapsed.as_millis(), elapsed.as_nanos())
    }
    fn to_index(&self, pos: Vector) -> usize {
        pos.y*self.x + pos.x
    }
    pub fn move_enemies(&mut self, player: Vector) {
        for index in 0..self.enemies.len() {
            let enemy = &self.enemies[index];
            if !enemy.active { continue }
            if !enemy.reachable { continue }
            if enemy.is_stunned() || enemy.is_windup() { continue }
            let mut new_pos = enemy.pos+self.backtraces[self.to_index(enemy.pos)].from;
            if self.has_collision(new_pos) || crate::random()&0b0001_1111 == 0 {
                let mut new_dir = match self.backtraces[self.to_index(enemy.pos)].from {
                    Direction::Up | Direction::Down => {
                        if bool::random() { Direction::Left }
                        else { Direction::Right }
                    }
                    Direction::Left | Direction::Right => {
                        if bool::random() { Direction::Up }
                        else { Direction::Down }
                    }
                };
                if match new_dir {
                    Direction::Up => {
                        if enemy.pos.y == 0 { true }
                        else { false }
                    }
                    Direction::Down => {
                        if enemy.pos.y == self.y-1 { true }
                        else { false }
                    }
                    Direction::Left => {
                        if enemy.pos.x == 0 { true }
                        else { false }
                    }
                    Direction::Right => {
                        if enemy.pos.x == self.x-1 { true }
                        else { false }
                    }
                } {
                    new_dir = self.backtraces[self.to_index(enemy.pos)].from;
                }
                new_pos = enemy.pos+new_dir;
            }
            if new_pos == player { continue }
            if self.has_collision(new_pos) { continue }
            self.enemies[index].pos = new_pos;
        }
    }
    pub fn make_room(&mut self, point_1: Vector, point_2: Vector) {
        for x in point_1.x..point_2.x {
            self[Vector::new(x, point_1.y)] = Some(Piece::Wall(Wall {}));
            self[Vector::new(x, point_2.y-1)] = Some(Piece::Wall(Wall {}));
        }
        for y in point_1.y..point_2.y {
            self[Vector::new(point_1.x, y)] = Some(Piece::Wall(Wall {}));
            self[Vector::new(point_2.x-1, y)] = Some(Piece::Wall(Wall {}));
        }
    }
    pub fn has_collision(&self, pos: Vector) -> bool {
        if let Some(piece) = self[pos] {
            if piece.has_collision() { return true }
        }
        for enemy in self.enemies.iter() {
            if enemy.pos == pos { return true }
        }
        false
    }
    pub fn contains_enemy(&self, pos: Vector) -> bool {
        for enemy in self.enemies.iter() {
            if enemy.pos == pos { return true }
        }
        false
    }
    pub fn dashable(&self, pos: Vector) -> bool {
        if let Some(piece) = self[pos] {
            if !piece.dashable() { return false }
        }
        true
    }
}
impl std::ops::Index<Vector> for Board {
    type Output = Option<Piece>;
    fn index(&self, index: Vector) -> &Self::Output {
        &self.inner[self.to_index(index)]
    }
}
impl std::ops::IndexMut<Vector> for Board {
    fn index_mut(&mut self, index: Vector) -> &mut Self::Output {
        &mut self.inner[index.y*self.x + index.x]
    }
}
#[derive(Copy, Clone)]
pub struct BackTrace {
    from: crate::Direction,
    pub cost: Option<usize>
}
impl BackTrace {
    const fn new() -> BackTrace {
        BackTrace {
            from: crate::Direction::Up,
            cost: None
        }
    }
}
#[derive(Clone, Copy)]
pub enum Piece {
    Wall(Wall),
    Door(Door),
}
impl Piece {
    fn render(&self, pos: Vector, board: &Board) -> (char, Option<Style>) {
        match self {
            Piece::Wall(_) => (Wall::render(pos, board), None),
            Piece::Door(door) => door.render(pos, board),
        }
    }
    fn has_collision(&self) -> bool {
        match self {
            Piece::Wall(_) => true,
            Piece::Door(door) => door.has_collision(),
        }
    }
    pub fn wall_connectable(&self) -> bool {
        match self {
            Piece::Wall(_) => true,
            Piece::Door(_) => true,
        }
    }
    fn dashable(&self) -> bool {
        match self {
            Piece::Wall(_) => false,
            Piece::Door(door) => !door.has_collision(),
        }
    }
}
struct PathData {
    from: Direction,
    cost: usize,
    heur: usize,
    pos: Vector
}
impl PathData {
    fn new(from: Direction, pos: Vector, target: Vector, cost: usize) -> PathData {
        PathData {
            from,
            cost,
            heur: cost + pos.x.abs_diff(target.x) + pos.y.abs_diff(target.y),
            pos
        }
    }
}
impl PartialEq for PathData {
    fn eq(&self, other: &Self) -> bool {
        self.heur == other.heur
    }
    fn ne(&self, other: &Self) -> bool {
        self.heur != other.heur
    }
}
impl Eq for PathData {}
impl PartialOrd for PathData {
    fn lt(&self, other: &Self) -> bool {
        self.heur.gt(&other.heur)
    }
    fn le(&self, other: &Self) -> bool {
        self.heur.ge(&other.heur)
    }
    fn gt(&self, other: &Self) -> bool {
        self.heur.lt(&other.heur)
    }
    fn ge(&self, other: &Self) -> bool {
        self.heur.le(&other.heur)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self < other { Some(std::cmp::Ordering::Less) }
        else if self > other { Some(std::cmp::Ordering::Greater) }
        else if self == other { Some(std::cmp::Ordering::Equal) }
        else { None }
    }
}
impl Ord for PathData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
    fn max(self, other: Self) -> Self
        where
            Self: Sized, {
        if self > other { self }
        else { other }
    }
    fn min(self, other: Self) -> Self
        where
            Self: Sized, {
        if self < other { self }
        else { other }
    }
    fn clamp(self, _min: Self, _max: Self) -> Self
        where
            Self: Sized, {
        unimplemented!("don't")
    }
}
struct Adj {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}
impl Adj {
    fn new(state: bool) -> Adj {
        Adj {
            up: state,
            down: state,
            left: state,
            right: state
        }
    }
}
