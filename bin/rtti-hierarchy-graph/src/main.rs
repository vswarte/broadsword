use std::io;
use std::env;
use std::fs::File;
use std::ops::Range;
use std::io::{Read, Write};

use broadsword::rtti;
use pelite::pe64::{Pe, PeFile};
use pelite::pe64::headers::SectionHeader;

mod graph;
mod symbol;
mod analysis;

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() != 2 {
        println!("Your invocation of this utility was incorrect. You little shit.");
        println!("This is how it's fucking done:");
        println!("$ ./rtti-extractor <exe file path>");
        return;
    }

    let path = &args[1];
    let mut file_handle = File::open(path).expect("Could not open file handle");

    let mut file_buffer = Vec::new();
    file_handle.read_to_end(&mut file_buffer).expect("Could not read file into buffer");
    let file_slice = file_buffer.as_slice();

    let pe = PeFile::from_bytes(file_slice)
        .expect("Could not parse file as PE file");

    let data = pe.section_headers()
        .by_name(".data")
        .expect("Could not find rdata section");

    let rdata = pe.section_headers()
        .by_name(".rdata")
        .expect("Could not find rdata section");

    let mut graph_edges = vec![];
    let mut type_infos = vec![];

    // Get the difference between the virtual and the file range so that we can rebase the IBOs
    // into file_slice offsets.
    let data_offset = data.virtual_range().start - data.file_range().start;
    let rdata_offset = rdata.virtual_range().start - rdata.file_range().start;

    let descriptors = locate_base_class_descriptors(file_slice, rdata, data);
    for descriptor in descriptors.iter() {
        let type_descriptor_offset = (descriptor.type_descriptor - data_offset) as usize;
        let type_descriptor = rtti::TypeDescriptor::from_slice(&file_slice[type_descriptor_offset..]);

        let type_info = symbol::TypeInfoSymbol::from(type_descriptor.name.as_str());

        let node_id = format!("{:x}", descriptor.type_descriptor);
        type_infos.push((node_id.clone(), type_info));

        let class_descriptor_offset = (descriptor.class_hierarchy_descriptor - rdata_offset) as usize;
        let class_descriptor = rtti::ClassHierarchyDescriptor::from_slice(&file_slice[class_descriptor_offset..]);

        // Check if the thing even has a parent
        if class_descriptor.base_class_count > 1 {
            // Rebase the IBO and get the second entry which should be the parent class
            let base_class_array_offset = class_descriptor.base_class_array.as_usize() as u32 - rdata_offset;

            // for i in 1..class_descriptor.base_class_count {
            for i in 1..2 {
                let parent_class_ibo_offset = (base_class_array_offset + 4 * i) as usize;

                // Fetch parent class IBO
                let parent_class_ibo_slice = &file_slice[parent_class_ibo_offset..parent_class_ibo_offset + 4];
                let parent_class_ibo = u32::from_le_bytes(parent_class_ibo_slice.try_into().unwrap());
                let parent_class_offset = (parent_class_ibo - rdata_offset) as usize;

                // Fetch base class descriptor for parent
                let parent_class_descriptor = rtti::BaseClassDescriptor::from_slice(&file_slice[parent_class_offset..]);

                graph_edges.push(graph::GraphEdge {
                    from: format!("{:x}", parent_class_descriptor.type_descriptor),
                    to: node_id.clone(),
                });
            }
        }
    }

    let root = analysis::map_into_tree(type_infos);

    let dotviz = graph::build_dotviz(root, graph_edges);
    io::stdout().write_all(dotviz.as_bytes()).unwrap();
}

/// Attempts to retrieve all RTTIBaseClassDescriptor instances it can find
fn locate_base_class_descriptors(
    buffer: &[u8],
    rdata: &SectionHeader,
    data: &SectionHeader,
) -> Vec<rtti::BaseClassDescriptor>  {
    let rdata_file_range = rdata.file_range();
    let rdata_file_range_usize = Range {
        start: rdata_file_range.start as usize,
        end: rdata_file_range.end as usize,
    };

    let rdata_virtual_range = rdata.virtual_range();
    let data_file_range = data.file_range();
    let data_virtual_range = data.virtual_range();

    rdata_file_range_usize.step_by(8)
        .filter_map(|p| {
            let candidate = rtti::BaseClassDescriptor::from_slice(&buffer[p..]);

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

            // Get the difference between the virtual and the file range so that we can rebase the IBOs
            // into byte offsets.
            let data_offset = data_virtual_range.start - data_file_range.start;
            let type_descriptor_offset = candidate.type_descriptor - data_offset;
            let type_descriptor = rtti::TypeDescriptor::from_slice(&buffer[type_descriptor_offset as usize..]);

            // TypeDescriptor must contain something that looks like a decorated symbol name
            if symbol::is_decorated_symbol(type_descriptor.name.as_str()) {
                return None;
            }

            Some(candidate)
        })
        .collect::<Vec<rtti::BaseClassDescriptor>>()
}
