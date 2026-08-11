use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisLength {
    /// A 64 x 64 grid
    Small = 0b100_0000,
    /// A 1024 x 1024 grid
    Full = 0b100_0000_0000,
}
// Yes this is less efficient, I don't care
impl ToBinary for AxisLength {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        self.to_inner().to_binary(binary)
    }
}
impl FromBinary for AxisLength {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self>
    where
        Self: Sized,
    {
        unsafe { Ok(AxisLength::from_inner(usize::from_binary(binary)?)) }
    }
}
impl std::str::FromStr for AxisLength {
    type Err = ();
    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Ok(match s {
            "small" => AxisLength::Small,
            "full" => AxisLength::Full,
            _ => return Err(()),
        })
    }
}
impl std::fmt::Display for AxisLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AxisLength::Small => "small",
                AxisLength::Full => "full",
            }
        )
    }
}
impl AxisLength {
    pub const fn to_inner(self) -> usize {
        // This is safe because we have guaranteed AxisLength to be a usize under the hood
        unsafe { std::mem::transmute(self) }
    }
    pub const fn as_inner(&self) -> &usize {
        // This is safe because self is &Self and not Self. If it was Self then the size would be
        // the same but it wouldn't be a reference
        unsafe { std::mem::transmute(self) }
    }
    /// It isn't my fault if you fuck this up
    pub const unsafe fn from_inner(inner: usize) -> Self {
        unsafe { std::mem::transmute(inner) }
    }
    pub const fn area(self) -> usize {
        self.to_inner().pow(2)
    }
}
impl PartialOrd for AxisLength {
    fn lt(&self, other: &Self) -> bool {
        self.to_inner().lt(other.as_inner())
    }
    fn le(&self, other: &Self) -> bool {
        self.to_inner().le(other.as_inner())
    }
    fn gt(&self, other: &Self) -> bool {
        self.to_inner().gt(other.as_inner())
    }
    fn ge(&self, other: &Self) -> bool {
        self.to_inner().ge(other.as_inner())
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.to_inner().partial_cmp(other.as_inner())
    }
}
impl Ord for AxisLength {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_inner().cmp(other.as_inner())
    }
    fn max(self, other: Self) -> Self {
        // We are piggybacking on the safety of the inputs
        unsafe { Self::from_inner(self.to_inner().max(other.to_inner())) }
    }
    fn min(self, other: Self) -> Self {
        // same as above
        unsafe { Self::from_inner(self.to_inner().min(other.to_inner())) }
    }
    fn clamp(self, min: Self, max: Self) -> Self {
        // same as above
        unsafe { Self::from_inner(self.to_inner().clamp(min.to_inner(), max.to_inner())) }
    }
}
#[cfg(test)]
mod tests {
    use super::AxisLength;
    use abes_nice_things::{FromBinary, ToBinary};
    use std::collections::VecDeque;
    #[test]
    fn small() {
        assert_eq!(64, AxisLength::Small.to_inner())
    }
    #[test]
    fn full() {
        assert_eq!(1024, AxisLength::Full.to_inner())
    }
    fn binary(axis: AxisLength) {
        let mut buf = VecDeque::new();
        axis.to_binary(&mut buf).unwrap();
        assert_eq!(axis, AxisLength::from_binary(&mut buf).unwrap())
    }
    #[test]
    fn small_binary() {
        binary(AxisLength::Small)
    }
    #[test]
    fn full_binary() {
        binary(AxisLength::Full)
    }
}
