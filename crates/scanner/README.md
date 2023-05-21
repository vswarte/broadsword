# Broadsword scanner

Offers constructs to scan things with. Currently, assumes in-memory ranges. It has two scanning methods built-in.

## Single-threaded scans
Example:
```rust
let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();

// Scannable is a &'static [u8] in which the pattern will be matched
let result = SimpleScanner::default().scan(scannable, &pattern).unwrap();
```

## Multi-threaded scans
Splits up the search array into N chunks, where N is the amount of available parallelism. It runs a simple scanner
per thread on the chunk assigned to the thread.

### Default (automatic thread count)
Example:
```rust
let result = ThreadedScanner::default().scan(scannable, &pattern);
```

### Manual thread count
Example:
```rust
let result = ThreadedScanner::new_with_thread_count(4).scan(scannable, &pattern);
```