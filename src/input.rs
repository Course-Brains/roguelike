use abes_nice_things::{FromBinary, ToBinary};
use std::io::Read;
pub enum Input {
    Arrow(Direction),           // move cursor
    Wasd(Direction, bool),      // move character
    Enter,                      // interact
    Attack,                     // attack
    Block,                      // block
    Return,                     // return cursor
    Wait,                       // do nothing
    SwapFocus,                  // swap camera focus(player/selector)
    Use(usize),                 // Use an item(1-6) or cast a spell (1-6)
    Convert,                    // Convert energy to money
    Aim,                        // Show a raytrace from the player
    Fast,                       // Toggle fast selector movement
    ClearFeedback,              // Clear the feedback
    Memorize,                   // Memorize the current position
    Remember,                   // Remember the memorized position
    AutoMove,                   // automatically move to the selected position
    ChangeRightColumn,          // rotate what is shown/used by the right column
    Delay(std::time::Duration), // Internal non player input for auto move
}
impl Input {
    pub fn get() -> Input {
        let mut lock = std::io::stdin().lock();
        let mut buf = [0_u8];
        loop {
            lock.read_exact(&mut buf).unwrap();
            match buf[0] {
                27 => {
                    // escape byte
                    lock.read_exact(&mut buf).unwrap();
                    lock.read_exact(&mut buf).unwrap(); // actual data
                    match buf[0] {
                        b'A' => return Input::Arrow(Direction::Up),
                        b'B' => return Input::Arrow(Direction::Down),
                        b'D' => return Input::Arrow(Direction::Left),
                        b'C' => return Input::Arrow(Direction::Right),
                        _ => {}
                    }
                }
                b'w' => return Input::Wasd(Direction::Up, false),
                b's' => return Input::Wasd(Direction::Down, false),
                b'a' => return Input::Wasd(Direction::Left, false),
                b'd' => return Input::Wasd(Direction::Right, false),
                b'W' => return Input::Wasd(Direction::Up, true),
                b'S' => return Input::Wasd(Direction::Down, true),
                b'A' => return Input::Wasd(Direction::Left, true),
                b'D' => return Input::Wasd(Direction::Right, true),
                b'q' => return Input::Attack,
                b'e' => return Input::Block,
                b'r' => return Input::Return,
                b't' => return Input::SwapFocus,
                b'\t' => return Input::Wait,
                b'\n' => return Input::Enter,
                b'1' => return Input::Use(1),
                b'2' => return Input::Use(2),
                b'3' => return Input::Use(3),
                b'4' => return Input::Use(4),
                b'5' => return Input::Use(5),
                b'6' => return Input::Use(6),
                b'c' => return Input::Convert,
                b'z' => return Input::Aim,
                b'f' => return Input::Fast,
                b'C' => return Input::ClearFeedback,
                b'm' => return Input::Memorize,
                b'R' => return Input::Remember,
                b'M' => return Input::AutoMove,
                b'x' => return Input::ChangeRightColumn,
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
impl std::ops::Not for Direction {
    type Output = Direction;
    fn not(self) -> Self::Output {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
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
impl FromBinary for Direction {
    fn from_binary(binary: &mut dyn Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Self::Up,
            1 => Self::Down,
            2 => Self::Left,
            3 => Self::Right,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not get Direction from binary",
                ));
            }
        })
    }
}
impl ToBinary for Direction {
    fn to_binary(&self, binary: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        match self {
            Self::Up => 0_u8.to_binary(binary)?,
            Self::Down => 1_u8.to_binary(binary)?,
            Self::Left => 2_u8.to_binary(binary)?,
            Self::Right => 3_u8.to_binary(binary)?,
        }
        Ok(())
    }
}
