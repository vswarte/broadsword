# Broadsword
A set of shitty memory hacking tools.

## DLL bootstrapping
Small macro to generate the DllMain:
```rust
use broadsword::dll;

#[dll::entrypoint]
pub fn entry(module_base: usize) -> bool {
  // Usual DllMain stuff
}
```

## Scanner
### Single-threaded scans
Example:
```rust
use broadsword::scanner::Pattern;
use broadsword::scanner::SimpleScanner;

let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();

// Scannable is a &'static [u8] in which the pattern will be matched
let result = SimpleScanner::default().scan(scannable, &pattern).unwrap();
```

### Multi-threaded scans
Splits up the search array into N chunks, where N is the amount of available parallelism. It runs a simple scanner
per thread on the chunk assigned to the thread.

#### Default (automatic thread count)
Example:
```rust
use broadsword::scanner::Pattern;
use broadsword::scanner::ThreadedScanner;

let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();
let result = ThreadedScanner::default().scan(scannable, &pattern);
```

#### Manual thread count
Example:
```rust
use broadsword::scanner::Pattern;
use broadsword::scanner::ThreadedScanner;

let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();
let result = ThreadedScanner::new_with_thread_count(4).scan(scannable, &pattern);
```

### Multiple matches
You can also use the scanners to match all occurrences of a pattern:

```rust
use broadsword::scanner::Pattern;
use broadsword::scanner::SimpleScanner;
use broadsword::scanner::ThreadedScanner;

let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();
let simple_result = SimpleScanner::default().scan_all(scannable, &pattern);
let threaded_result = ThreadedScanner::default().scan_all(scannable, &pattern);
```

### Captures
Both scanners also have the ability of capturing bytes from the occurrences by using the `[00 00 00 00]` notation where 
the square brackets indicate what should be captured.

```rust
use broadsword::scanner::Pattern;
use broadsword::scanner::SimpleScanner;

let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();
let result = SimpleScanner::default().scan(scannable, &pattern);

assert_eq!(result.captures[0].location, 867777);
assert_eq!(result.captures[0].bytes, vec![0xc6, 0xcf, 0xd8, 0x11]);
```

## Windows Modules

### Finding a module
`get_module_handle` gives us the base for a module based on the module name.
```rust
use broadsword::runtime::get_module_handle;

let game_base: usize = get_module_handle("eldenring.exe");
```

### Finding a symbol in a module
`get_module_symbol` find a function by examining the IAT.
```rust
use broadsword::runtime::get_module_symbol;

let create_file_w_ptr: usize = get_module_symbol("kernel32", "CreateFileW");
```

### Finding the module a pointer belongs to
`get_module_symbol` find a function by examining the IAT.
```rust
use broadsword::runtime::Module;
use broadsword::runtime::get_module_pointer_belongs_to;

let some_module: Module = get_module_pointer_belongs_to(0x123456).unwrap();
let module_name: String = some_module.name;
let module_memory_range: Range<usize> = some_module.memory_range;
```

### Finding the range of a section within a module
`get_module_symbol` find a function by examining the IAT.
```rust
use broadsword::runtime::Module;
use broadsword::runtime::get_module_section_range;

let range: Range<usize> = get_module_section_range("eldenring.exe", ".text").unwrap();
```

## RTTI

### Instance class names
We use `get_rtti_instance_classname` to retrieve the RTTI classname from a class instance.
```rust
use broadsword::runtime::get_rtti_instance_classname;

let ptr: usize = 0x123456;
let class_name: Option<String> = get_rtti_instance_classname(ptr);
```

### Vftable class names
`get_rtti_classname` will give use the name too but it expects the vftable pointer directly
instead of a pointer to a class instance.
```rust
use broadsword::runtime::get_rtti_classname;

let ptr: usize = 0x123456;
let class_name: Option<String> = get_rtti_classname(ptr);
```
