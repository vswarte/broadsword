pub use broadsword_microsoft_runtime::memory::remove_pageguard;
pub use broadsword_microsoft_runtime::memory::set_pageguard;

pub use broadsword_microsoft_runtime::module::Module;
pub use broadsword_microsoft_runtime::module::get_module_handle;
pub use broadsword_microsoft_runtime::module::get_module_symbol;
pub use broadsword_microsoft_runtime::module::get_module_pointer_belongs_to;

pub use broadsword_microsoft_runtime::pointer::is_valid_pointer;

pub use broadsword_microsoft_runtime::rtti::get_vftable_pointer;
pub use broadsword_microsoft_runtime::rtti::get_classname as get_rtti_classname;
pub use broadsword_microsoft_runtime::rtti::get_instance_classname as get_rtti_instance_classname;