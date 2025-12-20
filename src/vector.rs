use crate::*;
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Vector {
    pub x: usize,
    pub y: usize,
}
impl Vector {
    pub const fn new(x: usize, y: usize) -> Vector {
        Vector { x, y }
    }
    pub fn to_move(self) -> crossterm::cursor::MoveTo {
        crossterm::cursor::MoveTo(self.x as u16, self.y as u16)
    }
    pub fn clamp(self, bounds: std::ops::Range<Vector>) -> Vector {
        let mut out = self;
        if bounds.start.x > out.x {
            out.x = bounds.start.x
        } else if bounds.end.x < out.x {
            out.x = bounds.end.x
        }
        if bounds.start.y > out.y {
            out.y = bounds.start.y
        } else if bounds.end.y < out.y {
            out.y = bounds.end.y
        }
        out
    }
    pub fn is_near(self, other: Vector, range: usize) -> bool {
        self.x.abs_diff(other.x) < range && self.y.abs_diff(other.y) < range
    }
    pub fn up(self) -> Vector {
        Vector::new(self.x, self.y - 1)
    }
    pub fn down(self) -> Vector {
        Vector::new(self.x, self.y + 1)
    }
    pub fn down_mut(&mut self) -> &mut Self {
        self.y += 1;
        self
    }
    pub fn left(self) -> Vector {
        Vector::new(self.x - 1, self.y)
    }
    pub fn right(self) -> Vector {
        Vector::new(self.x + 1, self.y)
    }
    pub fn abs_diff(self, other: Vector) -> Vector {
        Vector {
            x: self.x.abs_diff(other.x),
            y: self.y.abs_diff(other.y),
        }
    }
    pub fn sum_axes(self) -> usize {
        self.x + self.y
    }
}
impl std::ops::Sub for Vector {
    type Output = Vector;
    fn sub(self, rhs: Self) -> Self::Output {
        abes_nice_things::debug!({
            if self.x < rhs.x || self.y < rhs.y {
                let escape = 0_u8;
                while escape == 0 {
                    crate::log!("Le fucked is UP");
                    // trap for making sure I can use a debugger before it crashes
                    crate::bell(None);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        });
        Vector {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl std::ops::Add for Vector {
    type Output = Vector;
    fn add(self, rhs: Self) -> Self::Output {
        Vector {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl std::fmt::Display for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}
impl PartialOrd for Vector {
    fn lt(&self, other: &Self) -> bool {
        self.x.lt(&other.x) && self.y.lt(&other.y)
    }
    fn le(&self, other: &Self) -> bool {
        self.x.le(&other.x) && self.y.le(&other.y)
    }
    fn gt(&self, other: &Self) -> bool {
        self.x.gt(&other.x) && self.y.le(&other.y)
    }
    fn ge(&self, other: &Self) -> bool {
        self.x.ge(&other.x) && self.y.le(&other.y)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self < other {
            Some(std::cmp::Ordering::Less)
        } else if self > other {
            Some(std::cmp::Ordering::Greater)
        } else if self == other {
            Some(std::cmp::Ordering::Equal)
        } else {
            None
        }
    }
}
impl FromBinary for Vector {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Vector::new(
            usize::from_binary(binary)?,
            usize::from_binary(binary)?,
        ))
    }
}
impl ToBinary for Vector {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.x.to_binary(binary)?;
        self.y.to_binary(binary)?;
        Ok(())
    }
}
