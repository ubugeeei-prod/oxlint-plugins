# Profiling

Performance regressions should be investigated with release-like artifacts.

Use the workspace profile tasks:

```sh
vp run profile bench
vp run profile rust
vp run profile js
```

Rust profiling uses Cargo's `profiling` profile: release optimization, debug symbols, and no symbol stripping.

JavaScript profiling writes Node CPU profiles into `profiles/`.

Rule crates can use the shared Rust profiler from `oxlint_plugins_carton`:

```rust
use oxlint_plugins_carton::{global_profiler, profile};

fn scan(source_text: &str) -> usize {
    profile!("rule.scan", {
        source_text.as_bytes().len()
    })
}

global_profiler().enable();
let _ = scan("const event = data.error;");
global_profiler().disable();
eprintln!("{}", global_profiler().summary());
```

The profiler keeps the disabled path to one relaxed atomic load in the `profile!` macro. Enabled profiling uses sharded `FastHashMap` storage and can record allocation pressure when a binary installs `ProfilingAllocator` as its global allocator.

Benchmark checks still run in CI with:

```sh
vp run bench:check
```

Profiling output is local investigation data and should not be committed.
