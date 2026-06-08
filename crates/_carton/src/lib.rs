#![doc = "Performance primitives shared by Rust-backed oxlint plugins."]
#![deny(unsafe_code)]

pub mod arena;
pub mod hash;
pub mod profiler;

pub use arena::{Allocator, ArenaBox, ArenaString, ArenaVec, BumpAllocator};
pub use compact_str::CompactString;
pub use ghost_cell::{GhostCell, GhostToken};
pub use hash::{FastBuildHasher, FastHashMap, FastHashSet};
pub use profiler::{
    AllocationSnapshot, CacheStats, CounterEntries, CounterEntry, CounterMetrics, CounterSummary,
    Metrics, ProfileEntries, ProfileEntry, ProfileGuard, ProfileSummary, Profiler,
    ProfilingAllocator, Timer, allocation_snapshot, global_profiler, reset_allocation_counters,
};
pub use smallvec::SmallVec;

#[cfg(test)]
mod tests {
    use super::{GhostCell, GhostToken};

    #[test]
    fn exposes_ghost_cell_for_branded_shared_state() {
        GhostToken::new(|mut token| {
            let value = GhostCell::new(1);
            *value.borrow_mut(&mut token) = 2;
            assert_eq!(*value.borrow(&token), 2);
        });
    }
}
