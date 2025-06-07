use crate::{Vector,
    Style,
    pieces::{wall::Wall, door::Door},
    input::Direction,
    Enemy,
};
use std::io::Write;
use std::collections::{BinaryHeap, HashSet};
use std::ops::Range;
pub struct Board {
    pub x: usize,
    pub y: usize,
    pub render_x: usize, // the offset from the player to the edge of the screen(think radius)
    pub render_y: usize,
    inner: Vec<Option<Piece>>,
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
        crossterm::queue!(lock,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        ).unwrap();
        let x_bound = base.x..base.x+(self.render_x*2);
        let y_bound = base.y..base.y+(self.render_y*2);
        for x in x_bound.clone() {
            for y in y_bound.clone() {
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
        for item in self.backtraces.iter_mut() {
            item.cost = None;
        }
        for enemy in self.enemies.iter() {
            let mut to_visit = BinaryHeap::new();
            let mut visited = HashSet::new();
            to_visit.push(PathData::new(Direction::Up, player, enemy.pos, 0));
            while let Some(path_data) = to_visit.pop() {
                let index = self.to_index(path_data.pos);
                if self.backtraces[index].cost.is_none_or(|cost| cost > path_data.cost) {
                    self.backtraces[index].cost = Some(path_data.cost);
                    self.backtraces[index].from = path_data.from;
                }
                if path_data.pos == enemy.pos { break }
                if visited.contains(&path_data.pos) { continue }
                visited.insert(path_data.pos);
                let adj = self.get_adjacent(path_data.pos);
                if adj&0b0000_0001 != 0 {
                    to_visit.push(PathData::new(
                        Direction::Down,
                        path_data.pos+Direction::Up,
                        enemy.pos,
                        path_data.cost+1
                    ));
                }
                if adj&0b0000_0010 != 0 {
                    to_visit.push(PathData::new(
                        Direction::Up,
                        path_data.pos+Direction::Down,
                        enemy.pos,
                        path_data.cost+1
                    ))
                }
                if adj&0b0000_0100 != 0 {
                    to_visit.push(PathData::new(
                        Direction::Right,
                        path_data.pos+Direction::Left,
                        enemy.pos,
                        path_data.cost+1
                    ))
                }
                if adj&0b0000_1000 != 0 {
                    to_visit.push(PathData::new(
                        Direction::Left,
                        path_data.pos+Direction::Right,
                        enemy.pos,
                        path_data.cost+1
                    ))
                }
            }
        }
    }
    fn get_adjacent(&self, pos: Vector) -> u8 {
        // 8th: up
        // 7th: down
        // 6th: left
        // 5th: right
        let mut out = 0b0000_1111;
        
        if pos.y == 0 { out &= 0b0000_1110 }
        else if let Some(piece) = self[pos+Direction::Up] {
           if piece.has_collision() { out &= 0b0000_1110 }
        }
        if pos.y == self.y-1 { out &= 0b0000_1101 }
        else if let Some(piece) = self[pos+Direction::Down] {
            if piece.has_collision() { out &= 0b0000_1101 }
        }
        if pos.x == 0 { out &= 0b0000_1011 }
        else if let Some(piece) = self[pos+Direction::Left] {
            if piece.has_collision() { out &= 0b0000_1011 }
        }
        if pos.x == self.x-1 { out &= 0b0000_0111 }
        else if let Some(piece) = self[pos+Direction::Right] {
            if piece.has_collision() { out &= 0b0000_0111 }
        }
        out
    }
    fn to_index(&self, pos: Vector) -> usize {
        pos.y*self.x + pos.x
    }
    pub fn move_enemies(&mut self, player: Vector) {
        for index in 0..self.enemies.len() {
            let enemy = &self.enemies[index];
            if !enemy.active { continue }
            if enemy.is_stunned() || enemy.is_windup() { continue }
            if self.backtraces[self.to_index(enemy.pos)].cost.is_none() { continue }
            let new_pos = enemy.pos+self.backtraces[self.to_index(enemy.pos)].from;
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
