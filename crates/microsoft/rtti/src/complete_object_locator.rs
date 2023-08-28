use std::fmt;

pub struct CompleteObjectLocator {
    pub signature: u32,
    pub offset: u32,
    pub cd_offset: u32,
    pub type_descriptor: u32,
    pub class_hierarchy_descriptor: u32,
}

impl CompleteObjectLocator {
    /// Constructs a CompleteObjectLocator from a u8 slice.
    /// WARNING: This function does not check if something actually is a valid CompleteObjectLocator.
    pub fn from_bytes(input: impl AsRef<[u8]>) -> Self {
        let input = input.as_ref();

        Self {
            signature: u32::from_le_bytes(input[0..4].try_into().unwrap()),
            offset: u32::from_le_bytes(input[4..8].try_into().unwrap()),
            cd_offset: u32::from_le_bytes(input[8..12].try_into().unwrap()),
            type_descriptor: u32::from_le_bytes(input[12..16].try_into().unwrap()),
            class_hierarchy_descriptor: u32::from_le_bytes(input[16..20].try_into().unwrap()),
        }
    }
}

impl fmt::Debug for CompleteObjectLocator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RTTICompleteObjectLocator")
            .field("signature", &format_args!("{:#x?}", self.signature))
            .field("offset", &format_args!("{:?}", self.offset))
            .field("cd_offset", &format_args!("{:?}", self.cd_offset))
            .field("type_descriptor", &format_args!("{:#x?}", self.type_descriptor))
            .field("class_hierarchy_descriptor", &format_args!("{:#x?}", self.class_hierarchy_descriptor))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::complete_object_locator::CompleteObjectLocator;

    #[test]
    fn from_works() {
        let col = CompleteObjectLocator::from_bytes(vec![
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x98, 0x8e, 0xc6, 0x03, 0x80, 0x71, 0x2e, 0x03
        ]);

        assert_eq!(col.signature, 0x1);
        assert_eq!(col.offset, 0x0);
        assert_eq!(col.cd_offset, 0x0);
        assert_eq!(col.type_descriptor, 0x3c68e98);
        assert_eq!(col.class_hierarchy_descriptor, 0x32e7180);
    }
}
