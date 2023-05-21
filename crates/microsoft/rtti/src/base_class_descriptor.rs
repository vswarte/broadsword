use std::fmt;

pub struct BaseClassDescriptor {
    pub type_descriptor: u32,
    pub contained_base_count: u32,
    // TODO: pmd
    pub attributes: u32,
    pub class_hierarchy_descriptor: u32,
}

impl BaseClassDescriptor {
    pub fn from_slice(input: &[u8]) -> Self {
        Self {
            type_descriptor: u32::from_le_bytes(input[0..4].try_into().unwrap()),
            contained_base_count: u32::from_le_bytes(input[4..8].try_into().unwrap()),
            // TODO: pmd
            attributes: u32::from_le_bytes(input[20..24].try_into().unwrap()),
            class_hierarchy_descriptor: u32::from_le_bytes(input[24..28].try_into().unwrap()),
        }
    }
}

impl fmt::Debug for BaseClassDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RTTIBaseClassDescriptor")
            .field("type_descriptor", &format_args!("{:#x?}", self.type_descriptor))
            .field("contained_base_count", &format_args!("{}", self.contained_base_count))
            .field("attributes", &format_args!("{:#x?}", self.attributes))
            .field("class_hierachy_descriptor", &format_args!("{:#x?}", self.class_hierarchy_descriptor))
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
