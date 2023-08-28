use std::fmt;

pub struct ClassHierarchyDescriptor {
    pub signature: u32,
    pub attributes: u32,
    pub base_class_count: u32,
    pub base_class_array: u32,
}

impl ClassHierarchyDescriptor {
    pub fn from_slice(input: impl AsRef<[u8]>) -> Self {
        let input = input.as_ref();

        Self {
            signature: u32::from_le_bytes(input[0..4].try_into().unwrap()),
            attributes: u32::from_le_bytes(input[4..8].try_into().unwrap()),
            base_class_count: u32::from_le_bytes(input[8..12].try_into().unwrap()),
            base_class_array: u32::from_le_bytes(input[12..16].try_into().unwrap()),
        }
    }
}

impl fmt::Debug for ClassHierarchyDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RTTIClassHierarchyDescriptor")
            .field("signature", &format_args!("{:#x?}", self.signature))
            .field("attributes", &format_args!("{:b}", self.attributes))
            .field("base_class_count", &format_args!("{}", self.base_class_count))
            .field("base_class_array", &format_args!("{:?}", self.base_class_array))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::class_hierarchy_descriptor::ClassHierarchyDescriptor;

    #[test]
    fn from_works() {
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x98, 0x71, 0x2e, 0x03];

        let chd = ClassHierarchyDescriptor::from_slice(bytes);

        assert_eq!(chd.signature, 0x0);
        assert_eq!(chd.attributes, 0x0);
        assert_eq!(chd.base_class_count, 0x4);
        assert_eq!(chd.base_class_array, 0x32e7198);
    }
}
