use std::io::Read;
thread_local! {
    static URANDOM: std::cell::RefCell<std::io::BufReader<std::fs::File>> =
        std::cell::RefCell::new(std::io::BufReader::new(std::fs::File::open("/dev/urandom").unwrap()));
}
/// Creates a random number between 0.5 and 1
pub fn random() -> f64 {
    let mut bits: u64 = 0b00111111_11100000_00000000_00000000_00000000_00000000_00000000_00000000;
    //                    ^ sign
    //                     ^^^^^^^ ^^^^ exponent
    // 52 bits of mantissa
    // nearest greater multiple of 8 is 56 aka 7 bytes
    let mask: u64 = 0b00000000_00001111_11111111_11111111_11111111_11111111_11111111_11111111;
    let mantissa = u64::random() & mask;
    bits |= mantissa;

    f64::from_bits(bits)
}
pub trait Random {
    fn random() -> Self;
}
macro_rules! int_helper {
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
        $(int_helper!($type);)*
    };
}
int_helper!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);
