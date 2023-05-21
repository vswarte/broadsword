use std::ops;
use std::fmt;
use std::fmt::Formatter;

#[derive(Eq, PartialEq, Clone, Copy)]
pub struct Base {
    pub value: usize,
}

impl From<usize> for Base {
    fn from(value: usize) -> Self {
        Self { value }
    }
}

impl ops::Add<&Offset> for &Base {
    type Output = Address;

    fn add(self, rhs: &Offset) -> Self::Output {
        Address::from(self.value + rhs.value)
    }
}

impl fmt::Debug for Base {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Address({:#x})", self.value)
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub struct Offset {
    pub value: usize,
}

impl Offset {
    pub fn move_by(&mut self, by: usize) {
        self.value += by;
    }

    pub fn as_usize(&self) -> usize {
        self.value
    }
}

impl ops::Add<&Offset> for &Offset {
    type Output = Offset;

    fn add(self, rhs: &Offset) -> Self::Output {
        Offset::from(self.value + rhs.value)
    }
}

impl From<i32> for Offset {
    fn from(value: i32) -> Self {
        Self { value: value as usize }
    }
}

impl From<u32> for Offset {
    fn from(value: u32) -> Self {
        Self { value: value as usize }
    }
}

impl From<usize> for Offset {
    fn from(value: usize) -> Self {
        Self { value }
    }
}

impl Into<usize> for Offset {
    fn into(self) -> usize {
        self.value
    }
}

impl fmt::Debug for Offset {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Offset(+{:#x})", self.value)
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub struct Address {
    pub value: usize,
}

impl Address {
    pub fn rebase(&self, from: &Base, to: &Base) -> Self {
        let new_value = (self.value - from.value) + to.value;
        Self { value: new_value }
    }

    pub fn as_usize(&self) -> usize {
        self.value
    }
}

impl From<usize> for Address {
    fn from(value: usize) -> Self {
        Self { value }
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Address({:#x})", self.value)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Base, Offset};

    #[test]
    fn we_can_add_a_base_and_an_offset() {
        let base: Base = Base::from(0x1000);
        let offset: Offset = Offset::from(0x100);
        let address = &base + &offset;

        assert_eq!(address.value, 0x1100);
    }

    #[test]
    fn we_can_rebase_an_address() {
        let base_a: Base = 0x1000.into();
        let base_b: Base = 0x2000.into();
        let offset: Offset = 0x100.into();
        let address = &base_a + &offset;
        let new_address = address.rebase(&base_a, &base_b);

        assert_eq!(new_address.value, 0x2100);
    }

    #[test]
    fn we_can_move_an_offset() {
        let mut offset: Offset = 0x100.into();
        offset.move_by(0x10);

        assert_eq!(offset.value, 0x110);
    }

    #[test]
    fn we_can_into_an_offset() {
        let offset_usize: usize= 0x100;
        let offset: Offset = offset_usize.into();
        let result: usize = offset.into();

        assert_eq!(result, offset_usize);
    }

    #[test]
    fn we_can_read_offset_as_usize() {
        let offset_usize: usize= 0x100;
        let offset: Offset = offset_usize.into();

        assert_eq!(offset.as_usize(), offset_usize);
    }
}
