use std::fmt;
use std::ffi::CStr;

use broadsword_address::Address;

pub struct TypeDescriptor {
    /// Points `type_info`'s vftable.
    pub vftable: Address,
    /// Spare field. Should always be 0x0 as it's unused.
    pub spare: usize,
    /// A decorated symbol name.
    pub name: String,
}

impl TypeDescriptor {
    pub fn from_slice(input: &[u8]) -> Self {
       let vftable = Address::from(usize::from_le_bytes(input[0..8].try_into().unwrap()));

        Self {
            vftable,
            spare: usize::from_le_bytes(input[8..16].try_into().unwrap()),
            name: CStr::from_bytes_until_nul(&input[16..input.len()]).unwrap().to_string_lossy().to_string()
        }
    }
}

impl fmt::Debug for TypeDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TypeDescriptor")
            .field("vftable", &format_args!("{:#x?}", self.vftable))
            .field("spare", &format_args!("{:#x?}", self.spare))
            .field("name", &self.name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::type_descriptor::TypeDescriptor;

    #[test]
    fn from_works() {
        let bytes = vec![0x20, 0x7a, 0x1f, 0x43, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2e, 0x3f, 0x41, 0x56, 0x43, 0x53, 0x47, 0x70, 0x61, 0x72, 0x61, 0x6d, 0x52, 0x65, 0x70, 0x6f, 0x73, 0x69, 0x74, 0x6f, 0x72, 0x79, 0x49, 0x6d, 0x70, 0x40, 0x43, 0x53, 0x40, 0x40, 0x00, 0x00, 0x00];

        let td = TypeDescriptor::from_slice(bytes.as_slice());

        assert_eq!(td.vftable.as_usize(), 0x1431f7a20);
        assert_eq!(td.spare, 0x0);
        assert_eq!(td.name, String::from(".?AVCSGparamRepositoryImp@CS@@"));
    }
}
