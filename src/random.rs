use crate::math::*;
use std::io::Read;
thread_local! {
    static URANDOM: std::cell::RefCell<std::io::BufReader<std::fs::File>> =
        std::cell::RefCell::new(std::io::BufReader::new(std::fs::File::open("/dev/urandom").unwrap()));
}
/// Creates a uniform random number between 1 and 2
///
/// Specifically this creates a 64 bit floating point number with an exponent of 0, a positive
/// sign and random data in the mantissa
pub fn random() -> f64 {
    let mut bits: u64 = 0b00111111_11110000_00000000_00000000_00000000_00000000_00000000_00000000;
    //                    ^ sign
    //                     ^^^^^^^ ^^^^ exponent
    // 52 bits of mantissa
    // nearest greater multiple of 8 is 56 aka 7 bytes
    let mask: u64 = 0b00000000_00001111_11111111_11111111_11111111_11111111_11111111_11111111;
    let mantissa = u64::random() & mask;
    bits |= mantissa;

    f64::from_bits(bits)
}

/// Create a random instance of self
pub trait Random {
    /// Get a random value from all possible states of the type. Because it is for all possible
    /// states, floats do not implement this. If you want a float, try [random]
    fn random() -> Self;
}
macro_rules! random_int_helper {
    ($type:ty) => {
        impl Random for $type {
            fn random() -> Self {
                URANDOM.with_borrow_mut(|urandom| {
                    let mut buf = [0; std::mem::size_of::<$type>()];
                    urandom.read_exact(&mut buf).unwrap();
                    <$type>::from_le_bytes(buf)
                })
            }
        }
    };
    ($($type:ty)*) => {
        $(random_int_helper!($type);)*
    };
}
random_int_helper!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

/// Create a random value from self
pub trait PickRandom {
    type Out;
    fn generate(&self) -> Self::Out;
}
impl<'a, T> PickRandom for &'a [T] {
    type Out = &'a T;
    fn generate(&self) -> Self::Out {
        &self[((random() - 1.0) * self.len() as f64) as usize]
    }
}
impl PickRandom for std::ops::Range<usize> {
    type Out = usize;
    fn generate(&self) -> Self::Out {
        ((random() - 1.0) * (self.end - self.start) as f64 + self.start as f64) as usize
    }
}
impl PickRandom for std::ops::RangeInclusive<usize> {
    type Out = usize;
    fn generate(&self) -> Self::Out {
        (*self.start()..*self.end() + 1).generate()
    }
}
impl PickRandom for Zone<usize> {
    type Out = Vector<usize>;
    fn generate(&self) -> Self::Out {
        Vector::new(
            (self.left()..=self.right()).generate(),
            (self.top()..=self.bottom()).generate(),
        )
    }
}
