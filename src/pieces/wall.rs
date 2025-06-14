use crate::{Board, Vector};
pub const QUAD: char = '╬';
const NOT_DOWN: char = '╩';
const NOT_UP: char = '╦';
const NOT_RIGHT: char = '╣';
const NOT_LEFT: char = '╠';
const UP_LEFT: char = '╝';
const UP_RIGHT: char = '╚';
const DOWN_LEFT: char = '╗';
const DOWN_RIGHT: char = '╔';
const HORIZONTAL: char = '═';
const VERTICAL: char = '║';
#[derive(Debug, Clone, Copy)]
pub struct Wall {}
impl Wall {
    pub fn render(pos: Vector, board: &Board) -> char {
        let mut up = false;
        let mut down = false;
        let mut left = false;
        let mut right = false;
        let mut count = 0;
        if pos.y != 0 {
            if let Some(piece) = &board[pos - Vector::new(0, 1)] {
                if piece.wall_connectable() {
                    up = true;
                    count += 1
                }
            }
        }
        if pos.y != board.y - 1 {
            if let Some(piece) = &board[pos + Vector::new(0, 1)] {
                if piece.wall_connectable() {
                    down = true;
                    count += 1
                }
            }
        }
        if pos.x != 0 {
            if let Some(piece) = &board[pos - Vector::new(1, 0)] {
                if piece.wall_connectable() {
                    left = true;
                    count += 1
                }
            }
        }
        if pos.x != board.x - 1 {
            if let Some(piece) = &board[pos + Vector::new(1, 0)] {
                if piece.wall_connectable() {
                    right = true;
                    count += 1
                }
            }
        }
        match count {
            4 => QUAD,
            3 => {
                if !up {
                    return NOT_UP;
                }
                if !left {
                    return NOT_LEFT;
                }
                if !down {
                    return NOT_DOWN;
                }
                NOT_RIGHT
            }
            2 => {
                if up {
                    if left {
                        return UP_LEFT;
                    }
                    if right {
                        return UP_RIGHT;
                    }
                    return VERTICAL;
                }
                if down {
                    if left {
                        return DOWN_LEFT;
                    }
                    return DOWN_RIGHT;
                }
                HORIZONTAL
            }
            /*1 => {
                if up||down {
                    return VERTICAL
                }
                HORIZONTAL
            }*/
            _ => unreachable!("RUH ROH RAGGY"),
        }
    }
}
