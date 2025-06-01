use crate::{Vector,
    Style,
    pieces::{wall::Wall, door::Door, enemy::Enemy},
    input::Direction
};
use std::io::Write;
use std::collections::VecDeque;
pub struct Board {
    pub x: usize,
    pub y: usize,
    inner: Vec<(Option<Piece>, BackTrace)>
}
impl Board {
    pub fn new(x: usize, y: usize) -> Board {
        let mut inner = Vec::with_capacity(x*y);
        inner.resize_with(x*y, || (None, BackTrace::new()));
        Board {
            x,
            y,
            inner
        }
    }
    // returns whether or not the cursor has a background behind it
    pub fn render(&self) {
        let mut lock = std::collections::VecDeque::new();
        crossterm::queue!(lock,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        ).unwrap();
        for x in 0..self.x {
            for y in 0..self.y {
                if let Some(piece) = &self[Vector::new(x, y)] {
                    let (ch, style) = piece.render(Vector::new(x,y), self);
                    crossterm::queue!(lock,
                        crossterm::cursor::MoveTo(x as u16, y as u16)
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
        // It may seem inefficient to have an intermediary buffer when stdout already
        // has one, but without this, there is a vsync type visual artifact
        std::io::stdout().write_all(lock.make_contiguous()).unwrap();
    }
    pub fn has_background(&self, pos: Vector) -> bool {
        if let Some(piece) = self[pos] {
            return piece.render(pos, self).1.is_some_and(|x| x.has_background() )
        }
        false
    }
    pub fn generate_nav_data(&mut self, player: Vector) {
        let mut to_visit = VecDeque::new();
        to_visit.push_front(player);
        for item in self.inner.iter_mut() {
            item.1.cost = None;
        }
        let index = self.to_index(player);
        self.inner[index].1.cost = Some(0);
        while let Some(pos) = to_visit.pop_back() {
            let adj = self.get_adjacent(pos);
            if adj & 0b0000_0001 == 0b0000_0001 {
                let index = self.to_index(pos+Direction::Up);
                if self.inner[index].1.cost.is_none() {
                    self.inner[index].1.cost = Some(self.inner[self.to_index(pos)].1.cost.unwrap()+1);
                    self.inner[index].1.from = Direction::Down;
                    to_visit.push_front(pos + Direction::Up)
                }
            }
            if adj & 0b0000_0010 == 0b0000_0010 {
                let index = self.to_index(pos+Direction::Down);
                if self.inner[index].1.cost.is_none() {
                    self.inner[index].1.cost = Some(self.inner[self.to_index(pos)].1.cost.unwrap()+1);
                    self.inner[index].1.from = Direction::Up;
                    to_visit.push_front(pos + Direction::Down)
                }
            }
            if adj & 0b0000_0100 == 0b0000_0100 {
                let index = self.to_index(pos+Direction::Left);
                if self.inner[index].1.cost.is_none() {
                    self.inner[index].1.cost = Some(self.inner[self.to_index(pos)].1.cost.unwrap()+1);
                    self.inner[index].1.from = Direction::Right;
                    to_visit.push_front(pos + Direction::Left)
                }
            }
            if adj & 0b0000_1000 == 0b0000_1000 {
                let index = self.to_index(pos+Direction::Right);
                if self.inner[index].1.cost.is_none() {
                    self.inner[index].1.cost = Some(self.inner[self.to_index(pos)].1.cost.unwrap()+1);
                    self.inner[index].1.from = Direction::Left;
                    to_visit.push_front(pos + Direction::Right)
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
        for x in 0..self.x {
            for y in 0..self.y {
                if let Some(Piece::Enemy(enemy)) = self[Vector::new(x,y)] {
                    if enemy.is_stunned() || enemy.is_windup() { continue }
                    let new_pos = Vector::new(x,y)+self.inner[
                        self.to_index(Vector::new(x,y))
                    ].1.from;
                    if new_pos == player { continue }
                    if self[new_pos].is_some() { continue }
                    self[new_pos] = self[Vector::new(x,y)];
                    self[Vector::new(x,y)] = None;
                }
            }
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
}
impl std::ops::Index<Vector> for Board {
    type Output = Option<Piece>;
    fn index(&self, index: Vector) -> &Self::Output {
        &self.inner[self.to_index(index)].0
    }
}
impl std::ops::IndexMut<Vector> for Board {
    fn index_mut(&mut self, index: Vector) -> &mut Self::Output {
        &mut self.inner[index.y*self.x + index.x].0
    }
}
struct BackTrace {
    from: crate::Direction,
    cost: Option<usize>
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
    Enemy(Enemy),
}
impl Piece {
    pub fn render(&self, pos: Vector, board: &Board) -> (char, Option<Style>) {
        match self {
            Piece::Wall(_) => (Wall::render(pos, board), None),
            Piece::Door(door) => door.render(pos, board),
            Piece::Enemy(enemy) => enemy.render()
        }
    }
    pub fn has_collision(&self) -> bool {
        match self {
            Piece::Wall(_) => true,
            Piece::Door(door) => door.has_collision(),
            Piece::Enemy(_) => true,
        }
    }
    pub fn wall_connectable(&self) -> bool {
        match self {
            Piece::Wall(_) => true,
            Piece::Door(_) => true,
            Piece::Enemy(_) => false,
        }
    }
    pub fn dashable(&self) -> bool {
        match self {
            Piece::Wall(_) => false,
            Piece::Door(door) => !door.has_collision(),
            Piece::Enemy(_) => true
        }
    }
}
