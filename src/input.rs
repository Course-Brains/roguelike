use std::io::Read;
pub enum Input {
    Arrow(Direction),      // move cursor
    WASD(Direction, bool), // move character
    Enter,                 // interact
    Attack,                // attack
    Block,                 // block
    Return,                // return cursor
    Wait,                  // do nothing
    SwapFocus,             // swap camera focus(player/selector)
    Item(usize),           // Use an item(1-6)
    Convert,               // Convert energy to money
}
impl Input {
    pub fn get() -> Input {
        let mut lock = std::io::stdin().lock();
        let mut buf = [0_u8];
        loop {
            lock.read(&mut buf).unwrap();
            match buf[0] {
                27 => {
                    // escape byte
                    lock.read(&mut buf).unwrap();
                    lock.read(&mut buf).unwrap(); // actual data
                    match buf[0] {
                        b'A' => return Input::Arrow(Direction::Up),
                        b'B' => return Input::Arrow(Direction::Down),
                        b'D' => return Input::Arrow(Direction::Left),
                        b'C' => return Input::Arrow(Direction::Right),
                        _ => {}
                    }
                }
                b'w' => return Input::WASD(Direction::Up, false),
                b's' => return Input::WASD(Direction::Down, false),
                b'a' => return Input::WASD(Direction::Left, false),
                b'd' => return Input::WASD(Direction::Right, false),
                b'W' => return Input::WASD(Direction::Up, true),
                b'S' => return Input::WASD(Direction::Down, true),
                b'A' => return Input::WASD(Direction::Left, true),
                b'D' => return Input::WASD(Direction::Right, true),
                b'q' => return Input::Attack,
                b'e' => return Input::Block,
                b'r' => return Input::Return,
                b't' => return Input::SwapFocus,
                b'\t' => return Input::Wait,
                b'\n' => return Input::Enter,
                b'1' => return Input::Item(1),
                b'2' => return Input::Item(2),
                b'3' => return Input::Item(3),
                b'4' => return Input::Item(4),
                b'5' => return Input::Item(5),
                b'6' => return Input::Item(6),
                b'c' => return Input::Convert,
                _ => {}
            }
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
impl std::ops::Add<Direction> for crate::Vector {
    type Output = crate::Vector;
    fn add(self, rhs: Direction) -> Self::Output {
        match rhs {
            Direction::Up => crate::Vector::new(self.x, self.y - 1),
            Direction::Down => crate::Vector::new(self.x, self.y + 1),
            Direction::Left => crate::Vector::new(self.x - 1, self.y),
            Direction::Right => crate::Vector::new(self.x + 1, self.y),
        }
    }
}
impl std::ops::AddAssign<Direction> for crate::Vector {
    fn add_assign(&mut self, rhs: Direction) {
        match rhs {
            Direction::Up => self.y -= 1,
            Direction::Down => self.y += 1,
            Direction::Left => self.x -= 1,
            Direction::Right => self.x += 1,
        }
    }
}
impl std::ops::Sub<Direction> for crate::Vector {
    type Output = crate::Vector;
    fn sub(self, rhs: Direction) -> Self::Output {
        match rhs {
            Direction::Up => crate::Vector::new(self.x, self.y + 1),
            Direction::Down => crate::Vector::new(self.x, self.y - 1),
            Direction::Left => crate::Vector::new(self.x + 1, self.y),
            Direction::Right => crate::Vector::new(self.x - 1, self.y),
        }
    }
}
impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Up => write!(f, "up"),
            Direction::Down => write!(f, "down"),
            Direction::Left => write!(f, "left"),
            Direction::Right => write!(f, "right"),
        }
    }
}
