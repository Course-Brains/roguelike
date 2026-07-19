use abes_nice_things::Integer;
use abes_nice_things::Number;
use abes_nice_things::PrimAs;
use abes_nice_things::PrimFrom;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vector<T: Number> {
    pub x: T,
    pub y: T,
}
impl<T: Number> Vector<T> {
    pub const ZERO: Vector<T> = Vector::new(T::ZERO, T::ZERO);
    pub const fn new(x: T, y: T) -> Vector<T> {
        Vector { x, y }
    }
    /// visually up, which is down. I know this is weird but it makes rendering SO much easier.
    /// It decreases y by 1
    pub fn up(self) -> Self {
        Vector {
            x: self.x,
            y: self.y - T::ONE,
        }
    }
    /// Visually down, which is up. Again, this doesn't make sense but makes rendering easier if we
    /// treat y: 0 as the top. This increases y by 1
    pub fn down(self) -> Self {
        Vector {
            x: self.x,
            y: self.y + T::ONE,
        }
    }
    /// Visually left, which is properly left. It decreases x by 1
    pub fn left(self) -> Self {
        Vector {
            x: self.x - T::ONE,
            y: self.y,
        }
    }
    /// Visually right which is proper right. It increases x by 1
    pub fn right(self) -> Self {
        Vector {
            x: self.x + T::ONE,
            y: self.y,
        }
    }
    /// Applies min to each axis individually, this does mean that the resulting [Vector] can be a
    /// mix between the two inputs.
    pub fn min(self, other: Vector<T>) -> Vector<T> {
        Vector {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }
}
impl<T: Number> std::ops::Sub for Vector<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Vector {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl<T: Number> std::fmt::Display for Vector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
impl<T: Number + PrimAs<U>, U: Number> PrimAs<Vector<U>> for Vector<T> {
    fn prim_as(self) -> Vector<U> {
        Vector {
            x: self.x.prim_as(),
            y: self.y.prim_as(),
        }
    }
}
impl<T: Number + PrimFrom<U>, U: Number> PrimFrom<Vector<U>> for Vector<T> {
    fn prim_from(src: Vector<U>) -> Self {
        Vector {
            x: T::prim_from(src.x),
            y: T::prim_from(src.y),
        }
    }
}
impl<T: Integer> std::ops::Add<Direction> for Vector<T> {
    type Output = Vector<T>;
    fn add(self, rhs: Direction) -> Self::Output {
        match rhs {
            Direction::Up => self.up(),
            Direction::Down => self.down(),
            Direction::Left => self.left(),
            Direction::Right => self.right(),
        }
    }
}
impl<T: Integer> std::ops::AddAssign<Direction> for Vector<T> {
    fn add_assign(&mut self, rhs: Direction) {
        *self = *self + rhs;
    }
}
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
/// A 2 dimensional area with INCLUSIVE BOUNDS
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zone<T: Number> {
    left: T,
    right: T,
    top: T,
    bottom: T,
}
impl<T: Number> Zone<T> {
    pub fn new(left: T, right: T, top: T, bottom: T) -> Option<Zone<T>> {
        if left > right || top > bottom {
            None
        } else {
            Some(Zone {
                left,
                right,
                top,
                bottom,
            })
        }
    }
    pub fn contains(&self, position: Vector<T>) -> bool {
        // Once again, y of 0 is the top to make certain things easier
        self.left <= position.x
            && self.right >= position.x
            && self.top <= position.y
            && self.bottom >= position.y
    }
    pub fn from_vectors(first: Vector<T>, second: Vector<T>) -> Zone<T> {
        let left = if first.x < second.x {
            first.x
        } else {
            second.x
        };
        let right = if first.x > second.x {
            first.x
        } else {
            second.x
        };
        let top = if first.y < second.y {
            first.y
        } else {
            second.y
        };
        let bottom = if first.y > second.y {
            first.y
        } else {
            second.y
        };
        Zone {
            left,
            right,
            top,
            bottom,
        }
    }
    pub fn width(&self) -> T {
        self.right - self.left + T::ONE
    }
    pub fn height(&self) -> T {
        self.bottom - self.top + T::ONE
    }
    pub fn top_left(&self) -> Vector<T> {
        Vector::new(self.left, self.top)
    }
    pub fn bottom_right(&self) -> Vector<T> {
        Vector::new(self.right, self.bottom)
    }
    pub fn left(&self) -> T {
        self.left
    }
    pub fn top(&self) -> T {
        self.top
    }
    pub fn right(&self) -> T {
        self.right
    }
    pub fn bottom(&self) -> T {
        self.bottom
    }
}
impl<T: Number> std::ops::Add<Vector<T>> for Zone<T> {
    type Output = Zone<T>;
    fn add(self, rhs: Vector<T>) -> Self::Output {
        Zone {
            left: self.left + rhs.x,
            right: self.right + rhs.x,
            top: self.top + rhs.y,
            bottom: self.bottom + rhs.y,
        }
    }
}
impl<T: Number> std::ops::Sub<Vector<T>> for Zone<T> {
    type Output = Zone<T>;
    fn sub(self, rhs: Vector<T>) -> Self::Output {
        Zone {
            left: self.left - rhs.x,
            right: self.right - rhs.x,
            top: self.top - rhs.y,
            bottom: self.bottom - rhs.y,
        }
    }
}
impl<T: Integer> Zone<T> {
    /// Creates an iterator over the area of the zone which goes left to right then down a line
    /// repeatedly and gives an additional bool which is true when it is the last position in a
    /// row.
    pub fn scanlines<'a>(&'a self) -> Scanlines<'a, T> {
        Scanlines {
            zone: self,
            position: Vector::new(self.left, self.top),
        }
    }
}
pub struct Scanlines<'a, T: Integer> {
    zone: &'a Zone<T>,
    position: Vector<T>,
}
impl<'a, T: Integer> Iterator for Scanlines<'a, T> {
    type Item = (Vector<T>, bool);
    fn next(&mut self) -> Option<Self::Item> {
        // Go to next line
        if self.position.x > self.zone.right {
            self.position.y += T::ONE;
            self.position.x = self.zone.left;
        }
        // Done
        if self.position.y > self.zone.bottom {
            return None;
        }
        let out = self.position;
        self.position.x += T::ONE;
        return Some((out, out.x >= self.zone.right));
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        // if done, then none
        if self.position.y > self.zone.top {
            return (0, Some(0));
        }

        let size = ((self.zone.top - self.position.y) * (self.zone.right - self.zone.left)
            + self.zone.right
            - self.position.x)
            .prim_as();
        (size, Some(size))
    }
}
#[cfg(test)]
#[test]
fn validate_scanlines() {
    let zone = Zone::new(0, 63, 0, 63).unwrap(); // 64x64 = 4096
    assert_eq!(zone.width(), 64);
    assert_eq!(zone.height(), 64);
    assert_eq!(zone.scanlines().count(), 4096)
}
