use std::ops;

struct Base {
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

struct Offset {
    pub value: usize,
}

impl From<usize> for Offset {
    fn from(value: usize) -> Self {
        Self { value }
    }
}

struct Address {
    pub value: usize,
}

impl Address {
    pub fn rebase(&self, from: &Base, to: &Base) -> Self {
        let new_value = (self.value - from.value) + to.value;
        Self { value: new_value }
    }
}

impl From<usize> for Address {
    fn from(value: usize) -> Self {
        Self { value }
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
}
