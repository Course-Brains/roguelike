use crate::{Vector, Style, pieces::wall::Wall, pieces::door::Door, pieces::enemy::Enemy};
use std::io::Write;
pub struct Board {
    pub x: usize,
    pub y: usize,
    inner: Vec<Option<Piece>>
}
impl Board {
    pub fn new(x: usize, y: usize) -> Board {
        let mut inner = Vec::with_capacity(x*y);
        inner.resize_with(x*y, || None);
        Board {
            x,
            y,
            inner
        }
    }
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
        &self.inner[index.y*self.x + index.x]
    }
}
impl std::ops::IndexMut<Vector> for Board {
    fn index_mut(&mut self, index: Vector) -> &mut Self::Output {
        &mut self.inner[index.y*self.x + index.x]
    }
}
#[derive(Clone, Copy)]
pub enum Piece {
    Wall(Wall),
    Door(Door),
    Enemy(Enemy),
}
impl Piece {
    fn render(&self, pos: Vector, board: &Board) -> (char, Option<Style>) {
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
