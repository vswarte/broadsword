use std::fmt;

pub struct BaseClassDescriptor {
    pub type_descriptor: u32,
    pub contained_base_count: u32,
    // TODO: pmd
    pub attributes: u32,
    pub class_hierarchy_descriptor: u32,
}

impl BaseClassDescriptor {
    pub fn from_slice(input: impl AsRef<[u8]>) -> Self {
        let input = input.as_ref();

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
            .field("class_hierarchy_descriptor", &format_args!("{:#x?}", self.class_hierarchy_descriptor))
            .finish()
    }
}