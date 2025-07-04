use std::sync::atomic::{AtomicU8, Ordering};
static RANDOM_TABLE: [u8; 256] = [
    0, 8, 109, 220, 222, 241, 155, 115, 75, 248, 245, 137, 16, 66, 74, 21, 209, 47, 80, 238, 154,
    27, 205, 130, 161, 89, 65, 36, 95, 110, 85, 48, 210, 142, 211, 240, 22, 67, 200, 50, 28, 188,
    52, 140, 208, 120, 68, 151, 62, 51, 184, 190, 91, 204, 152, 215, 149, 104, 25, 178, 252, 183,
    202, 182, 141, 197, 4, 81, 181, 242, 145, 23, 39, 227, 157, 207, 225, 193, 219, 97, 122, 179,
    249, 1, 175, 144, 55, 218, 29, 246, 167, 53, 169, 116, 191, 131, 2, 235, 10, 92, 9, 147, 138,
    77, 69, 172, 78, 176, 173, 213, 174, 119, 94, 158, 41, 30, 230, 49, 111, 164, 70, 35, 5, 37,
    171, 57, 132, 156, 11, 56, 42, 153, 133, 229, 73, 146, 64, 61, 102, 192, 135, 106, 38, 199,
    195, 86, 96, 203, 121, 101, 170, 247, 180, 113, 72, 250, 108, 7, 255, 237, 129, 226, 79, 107,
    112, 166, 103, 233, 24, 223, 239, 124, 198, 58, 60, 82, 128, 3, 185, 40, 143, 217, 148, 224,
    83, 206, 163, 45, 63, 90, 168, 114, 59, 33, 159, 99, 12, 139, 127, 100, 125, 196, 15, 44, 194,
    253, 54, 14, 117, 228, 71, 6, 160, 93, 186, 87, 244, 134, 20, 32, 123, 251, 26, 13, 17, 46, 34,
    231, 232, 76, 31, 221, 88, 18, 216, 165, 212, 105, 201, 234, 98, 43, 19, 177, 254, 150, 189,
    84, 118, 214, 187, 136, 126, 162, 236, 243,
];
static INDEX: AtomicU8 = AtomicU8::new(0);
pub fn random() -> u8 {
    RANDOM_TABLE[INDEX.fetch_add(1, Ordering::SeqCst) as usize]
}
pub fn initialize() {
    crate::log!(
        "Initialized with random index: {}",
        std::process::id() & 255
    );
    INDEX.store((std::process::id() & 0b1111_1111) as u8, Ordering::SeqCst)
}
pub fn initialize_with(index: u8) {
    INDEX.store(index, Ordering::SeqCst);
}
pub fn random_in_range(range: std::ops::Range<u8>) -> u8 {
    random() % (range.end - range.start) + range.start
}
pub fn random_index(max: usize) -> Option<usize> {
    match max > 256 {
        true => Some(random() as usize),
        false => {
            if max == 0 {
                return None;
            }
            Some(random() as usize % max)
        }
    }
}
pub trait Random {
    fn random() -> Self;
}
impl Random for bool {
    fn random() -> Self {
        random() & 0b0000_0001 == 1
    }
}
impl Random for crate::ItemType {
    fn random() -> Self {
        match random() & 0b0000_0001 {
            0 => Self::MageSight,
            1 => Self::HealthPotion,
            _ => unreachable!("idk, not my problem"),
        }
    }
}
impl Random for crate::upgrades::UpgradeType {
    fn random() -> Self {
        match random() & 0b0000_0000 {
            0 => Self::MageEye,
            _ => unreachable!("Le fucked is up"),
        }
    }
}
