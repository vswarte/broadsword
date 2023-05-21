use std::fmt;
use broadsword_address::Offset;

pub struct CompleteObjectLocator {
    pub signature: u32,
    pub offset: Offset,
    pub cd_offset: Offset,
    pub type_descriptor: Offset,
    pub class_hierarchy_descriptor: Offset,
}

impl CompleteObjectLocator {
    /// Constructs a CompleteObjectLocator from a u8 slice.
    /// WARNING: This function does not check if something actually is a valid CompleteObjectLocator.
    pub fn from_slice(input: &[u8]) -> Self {
        let offset = Offset::from(u32::from_le_bytes(input[4..8].try_into().unwrap()));
        let cd_offset = Offset::from(u32::from_le_bytes(input[8..12].try_into().unwrap()));
        let type_descriptor = Offset::from(u32::from_le_bytes(input[12..16].try_into().unwrap()));
        let class_hierarchy_descriptor = Offset::from(u32::from_le_bytes(input[16..20].try_into().unwrap()));

        Self {
            signature: u32::from_le_bytes(input[0..4].try_into().unwrap()),
            offset,
            cd_offset,
            type_descriptor,
            class_hierarchy_descriptor,
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
        let bytes = vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x98, 0x8e, 0xc6, 0x03, 0x80, 0x71, 0x2e, 0x03 ];

        let col = CompleteObjectLocator::from_slice(bytes.as_slice());

        assert_eq!(col.signature, 0x1);
        assert_eq!(col.offset.as_usize(), 0x0);
        assert_eq!(col.cd_offset.as_usize(), 0x0);
        assert_eq!(col.type_descriptor.as_usize(), 0x3c68e98);
        assert_eq!(col.class_hierarchy_descriptor.as_usize(), 0x32e7180);
    }
}
