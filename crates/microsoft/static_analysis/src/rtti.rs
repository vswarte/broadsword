use std::ops;

use std::collections::HashMap;
use pelite::pe64::headers::SectionHeader;
use broadsword_address::Offset;
use broadsword_rtti::symbol::is_decorated_symbol;
use broadsword_rtti::type_descriptor::TypeDescriptor;
use broadsword_rtti::base_class_descriptor::BaseClassDescriptor;

/// Attempts to retrieve all RTTIBaseClassDescriptor instances it can find
pub fn locate_base_class_descriptors(
    buffer: &[u8],
    rdata: &SectionHeader,
    data: &SectionHeader,
) -> HashMap<Offset, BaseClassDescriptor>  {
    let rdata_file_range = rdata.file_range();
    let rdata_file_range_usize = ops::Range {
        start: rdata_file_range.start as usize,
        end: rdata_file_range.end as usize,
    };

    let rdata_virtual_range = rdata.virtual_range();
    let data_file_range = data.file_range();
    let data_virtual_range = data.virtual_range();

    rdata_file_range_usize.step_by(8)
        .filter_map(|p| {
            let candidate = BaseClassDescriptor::from_slice(&buffer[p..]);

            if candidate.contained_base_count > 0x20 {
                return None;
            }

            // TypeDescriptor should be somewhere in .data
            if !data_virtual_range.contains(&candidate.type_descriptor) {
                return None;
            }

            // Class hierarchy descriptor should be somewhere in .rdata
            if !rdata_virtual_range.contains(&candidate.class_hierarchy_descriptor) {
                return None;
            }

            // Get the difference between the virtual and the file range so that we can rebase the
            // IBOs into byte offsets.
            let data_offset = data_virtual_range.start - data_file_range.start;
            let type_descriptor_offset = candidate.type_descriptor - data_offset;
            let type_descriptor = TypeDescriptor::from_slice(&buffer[type_descriptor_offset as usize..]);

            // TypeDescriptor must contain something that looks like a decorated symbol name
            if !is_decorated_symbol(type_descriptor.name.as_str()) {
                return None;
            }

            Some((Offset::from(p), candidate))
        })
        .collect()
}
