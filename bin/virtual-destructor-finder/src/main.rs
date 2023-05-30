use std::env;
use std::fs::File;
use std::io::Read;

use broadsword::rtti;
use broadsword::address;
use pelite::pe64::{Pe, PeFile};
use broadsword::static_analysis::locate_base_class_descriptors;

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() != 2 {
        println!("Your invocation of this utility was incorrect. Specify an file to analyze.");
        println!("$ ./virtual-destructor-finder <exe file path>");
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

    // Get the difference between the virtual and the file range so that we can rebase the IBOs
    // into file_slice offsets.
    let data_offset = data.virtual_range().start - data.file_range().start;

    let descriptors = locate_base_class_descriptors(file_slice, rdata, data);
    for descriptor in descriptors.iter() {
        let type_descriptor_offset = (descriptor.1.type_descriptor - data_offset) as usize;
        let type_descriptor = rtti::TypeDescriptor::from_slice(&file_slice[type_descriptor_offset..]);

        let vftable_address = type_descriptor.vftable;
        if vftable_address != address::Address::from(0x1431f7a20) {
            println!("{:#?}", vftable_address);
        }
    }
}