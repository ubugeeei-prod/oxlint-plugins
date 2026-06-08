#![doc = "Low-overhead profiling utilities for rule and NAPI hot paths."]
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

use crate::{FastHashMap, SmallVec};

const PROFILER_SHARDS: usize = 32;
const PROFILE_HISTOGRAM_BUCKETS: usize = 48;

type MetricsMap = FastHashMap<&'static str, Metrics>;
type CounterMap = FastHashMap<&'static str, CounterMetrics>;

pub type ProfileEntries = SmallVec<[ProfileEntry; 64]>;
pub type CounterEntries = SmallVec<[CounterEntry; 64]>;

thread_local! {
    static PROFILE_STACK: RefCell<SmallVec<[ProfileFrame; 16]>> = RefCell::new(SmallVec::new());
    static ALLOCATION_TRACKING_SUPPRESSION: Cell<usize> = const { Cell::new(0) };
}

static ALLOCATION_TRACKING_ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static ALLOC_ZEROED_CALLS: AtomicU64 = AtomicU64::new(0);
static ALLOC_FAILURES: AtomicU64 = AtomicU64::new(0);
static ALLOC_ZEROED_FAILURES: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static ALLOC_ZEROED_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static DEALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static REALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static REALLOC_FAILURES: AtomicU64 = AtomicU64::new(0);
static REALLOC_OLD_BYTES: AtomicU64 = AtomicU64::new(0);
static REALLOC_NEW_BYTES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct AllocationTrackingPause;

impl Drop for AllocationTrackingPause {
    fn drop(&mut self) {
        ALLOCATION_TRACKING_SUPPRESSION.with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
}

#[inline]
fn pause_allocation_tracking() -> AllocationTrackingPause {
    ALLOCATION_TRACKING_SUPPRESSION.with(|depth| {
        depth.set(depth.get().saturating_add(1));
    });
    AllocationTrackingPause
}

#[inline]
fn allocation_tracking_is_suppressed() -> bool {
    ALLOCATION_TRACKING_SUPPRESSION
        .try_with(|depth| depth.get() > 0)
        .unwrap_or(false)
}

#[inline]
fn allocation_tracking_is_enabled() -> bool {
    ALLOCATION_TRACKING_ENABLED.load(Ordering::Relaxed) && !allocation_tracking_is_suppressed()
}

#[derive(Debug)]
struct ProfileFrame {
    name: &'static str,
    start: Instant,
    child_duration: Duration,
}

#[derive(Debug)]
pub struct ProfileGuard {
    profiler: &'static Profiler,
}

impl ProfileGuard {
    #[inline]
    fn start(profiler: &'static Profiler, name: &'static str) -> Self {
        let _allocation_tracking = pause_allocation_tracking();

        PROFILE_STACK.with(|stack| {
            stack.borrow_mut().push(ProfileFrame {
                name,
                start: Instant::now(),
                child_duration: Duration::ZERO,
            });
        });

        Self { profiler }
    }
}

impl Drop for ProfileGuard {
    fn drop(&mut self) {
        PROFILE_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            let Some(frame) = stack.pop() else {
                return;
            };

            let duration = frame.start.elapsed();
            if let Some(parent) = stack.last_mut() {
                parent.child_duration += duration;
            }

            self.profiler
                .record_sample_enabled(frame.name, duration, frame.child_duration);
        });
    }
}

#[derive(Debug)]
pub struct Timer {
    start: Instant,
    name: &'static str,
}

impl Timer {
    #[inline]
    pub fn start(name: &'static str) -> Self {
        Self {
            start: Instant::now(),
            name,
        }
    }

    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    #[inline]
    pub fn stop(self) -> Duration {
        self.elapsed()
    }

    #[inline]
    pub fn record(self, profiler: &Profiler) {
        profiler.record(self.name, self.elapsed());
    }
}

#[derive(Debug, Clone)]
pub struct Metrics {
    pub count: u64,
    pub total_duration: Duration,
    pub self_duration: Duration,
    pub child_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub min_self_duration: Duration,
    pub max_self_duration: Duration,
    histogram: [u64; PROFILE_HISTOGRAM_BUCKETS],
    samples_over_1ms: u64,
    samples_over_10ms: u64,
    samples_over_100ms: u64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            count: 0,
            total_duration: Duration::ZERO,
            self_duration: Duration::ZERO,
            child_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            min_self_duration: Duration::MAX,
            max_self_duration: Duration::ZERO,
            histogram: [0; PROFILE_HISTOGRAM_BUCKETS],
            samples_over_1ms: 0,
            samples_over_10ms: 0,
            samples_over_100ms: 0,
        }
    }

    pub fn record(&mut self, duration: Duration) {
        self.record_with_child(duration, Duration::ZERO);
    }

    pub fn record_with_child(&mut self, duration: Duration, child_duration: Duration) {
        let self_duration = duration.saturating_sub(child_duration);

        self.count = self.count.saturating_add(1);
        self.total_duration += duration;
        self.self_duration += self_duration;
        self.child_duration += child_duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.min_self_duration = self.min_self_duration.min(self_duration);
        self.max_self_duration = self.max_self_duration.max(self_duration);
        self.histogram[duration_bucket(duration)] =
            self.histogram[duration_bucket(duration)].saturating_add(1);

        if duration >= Duration::from_millis(1) {
            self.samples_over_1ms = self.samples_over_1ms.saturating_add(1);
        }
        if duration >= Duration::from_millis(10) {
            self.samples_over_10ms = self.samples_over_10ms.saturating_add(1);
        }
        if duration >= Duration::from_millis(100) {
            self.samples_over_100ms = self.samples_over_100ms.saturating_add(1);
        }
    }

    pub fn average(&self) -> Duration {
        average_duration(self.total_duration, self.count)
    }

    pub fn self_average(&self) -> Duration {
        average_duration(self.self_duration, self.count)
    }

    pub fn percentile(&self, percentile: f64) -> Duration {
        if self.count == 0 {
            return Duration::ZERO;
        }

        let target = ((self.count as f64) * percentile.clamp(0.0, 1.0)).ceil() as u64;
        let target = target.max(1);
        let mut seen = 0u64;

        for (index, count) in self.histogram.iter().enumerate() {
            seen = seen.saturating_add(*count);
            if seen >= target {
                return bucket_upper_bound(index);
            }
        }

        bucket_upper_bound(PROFILE_HISTOGRAM_BUCKETS - 1)
    }

    pub fn samples_over_1ms(&self) -> u64 {
        self.samples_over_1ms
    }

    pub fn samples_over_10ms(&self) -> u64 {
        self.samples_over_10ms
    }

    pub fn samples_over_100ms(&self) -> u64 {
        self.samples_over_100ms
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CounterMetrics {
    pub samples: u64,
    pub total: u64,
    pub min: u64,
    pub max: u64,
}

impl CounterMetrics {
    pub fn new() -> Self {
        Self {
            samples: 0,
            total: 0,
            min: u64::MAX,
            max: 0,
        }
    }

    pub fn record(&mut self, value: u64) {
        self.samples = self.samples.saturating_add(1);
        self.total = self.total.saturating_add(value);
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    pub fn average(&self) -> f64 {
        if self.samples == 0 {
            0.0
        } else {
            self.total as f64 / self.samples as f64
        }
    }
}

impl Default for CounterMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Profiler {
    metrics: [RwLock<MetricsMap>; PROFILER_SHARDS],
    counters: [RwLock<CounterMap>; PROFILER_SHARDS],
    enabled: AtomicBool,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            metrics: std::array::from_fn(|_| RwLock::new(FastHashMap::default())),
            counters: std::array::from_fn(|_| RwLock::new(FastHashMap::default())),
            enabled: AtomicBool::new(false),
        }
    }

    pub fn enabled() -> Self {
        let profiler = Self::new();
        profiler.enable();
        profiler
    }

    pub fn enable(&self) {
        reset_allocation_counters();
        ALLOCATION_TRACKING_ENABLED.store(true, Ordering::Relaxed);
        self.enabled.store(true, Ordering::Relaxed);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        ALLOCATION_TRACKING_ENABLED.store(false, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn timer(&self, name: &'static str) -> Option<Timer> {
        if self.is_enabled() {
            Some(Timer::start(name))
        } else {
            None
        }
    }

    pub fn record(&self, name: &'static str, duration: Duration) {
        if self.is_enabled() {
            self.record_enabled(name, duration);
        }
    }

    #[doc(hidden)]
    pub fn record_enabled(&self, name: &'static str, duration: Duration) {
        self.record_sample_enabled(name, duration, Duration::ZERO);
    }

    #[inline]
    pub fn global_span(&'static self, name: &'static str) -> Option<ProfileGuard> {
        if self.is_enabled() {
            Some(ProfileGuard::start(self, name))
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub fn record_sample_enabled(
        &self,
        name: &'static str,
        duration: Duration,
        child_duration: Duration,
    ) {
        let _allocation_tracking = pause_allocation_tracking();
        let mut metrics = self.metrics_write(Self::shard_index(name));
        metrics
            .entry(name)
            .or_default()
            .record_with_child(duration, child_duration);
    }

    pub fn record_counter(&self, name: &'static str, value: u64) {
        if self.is_enabled() {
            self.record_counter_enabled(name, value);
        }
    }

    #[doc(hidden)]
    pub fn record_counter_enabled(&self, name: &'static str, value: u64) {
        let _allocation_tracking = pause_allocation_tracking();
        let mut counters = self.counters_write(Self::shard_index(name));
        counters.entry(name).or_default().record(value);
    }

    pub fn record_fs_read(&self, bytes: usize) {
        if !self.is_enabled() {
            return;
        }

        self.record_counter_enabled("io.read.calls", 1);
        self.record_counter_enabled("io.read.bytes", usize_to_u64(bytes));
        self.record_counter_enabled("syscall.fs.read.calls", 1);
    }

    pub fn record_fs_read_failure(&self) {
        if !self.is_enabled() {
            return;
        }

        self.record_counter_enabled("io.read.calls", 1);
        self.record_counter_enabled("io.read.failures", 1);
        self.record_counter_enabled("syscall.fs.read.calls", 1);
        self.record_counter_enabled("syscall.fs.read.failures", 1);
    }

    pub fn record_fs_write(&self, bytes: usize) {
        if !self.is_enabled() {
            return;
        }

        self.record_counter_enabled("io.write.calls", 1);
        self.record_counter_enabled("io.write.attempted_bytes", usize_to_u64(bytes));
        self.record_counter_enabled("io.write.bytes", usize_to_u64(bytes));
        self.record_counter_enabled("syscall.fs.write.calls", 1);
    }

    pub fn record_fs_write_failure(&self, bytes: usize) {
        if !self.is_enabled() {
            return;
        }

        self.record_counter_enabled("io.write.calls", 1);
        self.record_counter_enabled("io.write.attempted_bytes", usize_to_u64(bytes));
        self.record_counter_enabled("io.write.failures", 1);
        self.record_counter_enabled("syscall.fs.write.calls", 1);
        self.record_counter_enabled("syscall.fs.write.failures", 1);
    }

    pub fn get(&self, name: &str) -> Option<Metrics> {
        self.metrics_read(Self::shard_index(name))
            .get(name)
            .cloned()
    }

    pub fn all(&self) -> FastHashMap<&'static str, Metrics> {
        let _allocation_tracking = pause_allocation_tracking();
        let mut all = FastHashMap::default();

        for shard in &self.metrics {
            let metrics = shard
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            all.extend(
                metrics
                    .iter()
                    .map(|(name, metrics)| (*name, metrics.clone())),
            );
        }

        all
    }

    pub fn clear(&self) {
        let _allocation_tracking = pause_allocation_tracking();

        for shard in &self.metrics {
            shard
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clear();
        }

        for shard in &self.counters {
            shard
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clear();
        }
    }

    pub fn summary(&self) -> ProfileSummary {
        let _allocation_tracking = pause_allocation_tracking();
        let mut entries = ProfileEntries::new();

        for shard in &self.metrics {
            let metrics = shard
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            entries.reserve(metrics.len());
            entries.extend(metrics.iter().map(|(name, metrics)| ProfileEntry {
                name,
                count: metrics.count,
                total: metrics.total_duration,
                self_total: metrics.self_duration,
                child_total: metrics.child_duration,
                average: metrics.average(),
                self_average: metrics.self_average(),
                min: metrics.min_duration,
                max: metrics.max_duration,
                self_min: metrics.min_self_duration,
                self_max: metrics.max_self_duration,
                p50: metrics.percentile(0.50),
                p95: metrics.percentile(0.95),
                p99: metrics.percentile(0.99),
                samples_over_1ms: metrics.samples_over_1ms(),
                samples_over_10ms: metrics.samples_over_10ms(),
                samples_over_100ms: metrics.samples_over_100ms(),
            }));
        }

        entries.sort_by_key(|entry| std::cmp::Reverse(entry.total));

        ProfileSummary { entries }
    }

    pub fn counter_summary(&self) -> CounterSummary {
        let _allocation_tracking = pause_allocation_tracking();
        let mut entries = CounterEntries::new();

        for shard in &self.counters {
            let counters = shard
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            entries.reserve(counters.len());
            entries.extend(counters.iter().map(|(name, counter)| CounterEntry {
                name,
                samples: counter.samples,
                total: counter.total,
                average: counter.average(),
                min: if counter.samples == 0 { 0 } else { counter.min },
                max: counter.max,
            }));
        }

        entries.sort_by(|left, right| left.name.cmp(right.name));

        CounterSummary { entries }
    }

    #[inline]
    fn metrics_read(&self, shard: usize) -> RwLockReadGuard<'_, MetricsMap> {
        self.metrics[shard]
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[inline]
    fn metrics_write(&self, shard: usize) -> RwLockWriteGuard<'_, MetricsMap> {
        self.metrics[shard]
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[inline]
    fn counters_write(&self, shard: usize) -> RwLockWriteGuard<'_, CounterMap> {
        self.counters[shard]
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[inline]
    fn shard_index(name: &str) -> usize {
        debug_assert!(PROFILER_SHARDS.is_power_of_two());

        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in name.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }

        (hash as usize) & (PROFILER_SHARDS - 1)
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ProfileSummary {
    pub entries: ProfileEntries,
}

impl ProfileSummary {
    pub fn has_slow_operations(&self, threshold: Duration) -> bool {
        self.entries.iter().any(|entry| entry.average > threshold)
    }

    pub fn slow_operations(&self, threshold: Duration) -> SmallVec<[&ProfileEntry; 16]> {
        self.entries
            .iter()
            .filter(|entry| entry.average > threshold)
            .collect()
    }
}

impl Display for ProfileSummary {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "Profile Summary:")?;
        writeln!(
            f,
            "{:<30} {:>8} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
            "Operation", "Count", "Total ms", "Self ms", "Avg ms", "P95 ms", "Min ms", "Max ms"
        )?;
        write_horizontal_rule(f, 114)?;

        for entry in &self.entries {
            writeln!(
                f,
                "{:<30} {:>8} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3}",
                entry.name,
                entry.count,
                duration_ms(entry.total),
                duration_ms(entry.self_total),
                duration_ms(entry.average),
                duration_ms(entry.p95),
                duration_ms(entry.min),
                duration_ms(entry.max)
            )?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CounterSummary {
    pub entries: CounterEntries,
}

impl CounterSummary {
    pub fn total(&self, name: &str) -> u64 {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.total)
            .unwrap_or(0)
    }

    pub fn total_matching(&self, prefix: &str, suffix: &str) -> u64 {
        self.entries
            .iter()
            .filter(|entry| entry.name.starts_with(prefix) && entry.name.ends_with(suffix))
            .map(|entry| entry.total)
            .sum()
    }
}

#[derive(Debug)]
pub struct CounterEntry {
    pub name: &'static str,
    pub samples: u64,
    pub total: u64,
    pub average: f64,
    pub min: u64,
    pub max: u64,
}

#[derive(Debug)]
pub struct ProfileEntry {
    pub name: &'static str,
    pub count: u64,
    pub total: Duration,
    pub self_total: Duration,
    pub child_total: Duration,
    pub average: Duration,
    pub self_average: Duration,
    pub min: Duration,
    pub max: Duration,
    pub self_min: Duration,
    pub self_max: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub samples_over_1ms: u64,
    pub samples_over_10ms: u64,
    pub samples_over_100ms: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AllocationSnapshot {
    pub alloc_calls: u64,
    pub alloc_zeroed_calls: u64,
    pub alloc_failures: u64,
    pub alloc_zeroed_failures: u64,
    pub alloc_bytes: u64,
    pub alloc_zeroed_bytes: u64,
    pub dealloc_calls: u64,
    pub dealloc_bytes: u64,
    pub realloc_calls: u64,
    pub realloc_failures: u64,
    pub realloc_old_bytes: u64,
    pub realloc_new_bytes: u64,
}

impl AllocationSnapshot {
    pub fn allocation_calls(&self) -> u64 {
        self.alloc_calls
            .saturating_add(self.alloc_zeroed_calls)
            .saturating_add(self.realloc_calls)
    }

    pub fn allocation_failures(&self) -> u64 {
        self.alloc_failures
            .saturating_add(self.alloc_zeroed_failures)
            .saturating_add(self.realloc_failures)
    }

    pub fn requested_bytes(&self) -> u64 {
        self.alloc_bytes
            .saturating_add(self.alloc_zeroed_bytes)
            .saturating_add(self.realloc_new_bytes)
    }

    pub fn released_bytes(&self) -> u64 {
        self.dealloc_bytes.saturating_add(self.realloc_old_bytes)
    }

    pub fn net_bytes(&self) -> i128 {
        i128::from(self.requested_bytes()) - i128::from(self.released_bytes())
    }

    pub fn requested_bytes_per_call(&self) -> f64 {
        let calls = self.allocation_calls();
        if calls == 0 {
            0.0
        } else {
            self.requested_bytes() as f64 / calls as f64
        }
    }
}

#[derive(Debug)]
pub struct ProfilingAllocator<A = System> {
    inner: A,
}

impl ProfilingAllocator<System> {
    pub const fn new() -> Self {
        Self { inner: System }
    }
}

impl Default for ProfilingAllocator<System> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> ProfilingAllocator<A> {
    pub const fn from_allocator(inner: A) -> Self {
        Self { inner }
    }
}

// SAFETY: Every method delegates to the wrapped allocator with the original
// layout and pointer arguments, then updates lock-free counters after the
// allocator returns.
unsafe impl<A: GlobalAlloc> GlobalAlloc for ProfilingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: This forwards the caller-provided layout to the wrapped allocator.
        let ptr = unsafe { self.inner.alloc(layout) };

        if allocation_tracking_is_enabled() {
            if ptr.is_null() {
                ALLOC_FAILURES.fetch_add(1, Ordering::Relaxed);
            } else {
                ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
                ALLOC_BYTES.fetch_add(usize_to_u64(layout.size()), Ordering::Relaxed);
            }
        }

        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: This forwards the caller-provided layout to the wrapped allocator.
        let ptr = unsafe { self.inner.alloc_zeroed(layout) };

        if allocation_tracking_is_enabled() {
            if ptr.is_null() {
                ALLOC_ZEROED_FAILURES.fetch_add(1, Ordering::Relaxed);
            } else {
                ALLOC_ZEROED_CALLS.fetch_add(1, Ordering::Relaxed);
                ALLOC_ZEROED_BYTES.fetch_add(usize_to_u64(layout.size()), Ordering::Relaxed);
            }
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if allocation_tracking_is_enabled() {
            DEALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            DEALLOC_BYTES.fetch_add(usize_to_u64(layout.size()), Ordering::Relaxed);
        }

        // SAFETY: This forwards the caller-provided pointer and layout to the wrapped allocator.
        unsafe { self.inner.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: This forwards the caller-provided pointer, layout, and new size.
        let new_ptr = unsafe { self.inner.realloc(ptr, layout, new_size) };

        if allocation_tracking_is_enabled() {
            if new_ptr.is_null() {
                REALLOC_FAILURES.fetch_add(1, Ordering::Relaxed);
            } else {
                REALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
                REALLOC_OLD_BYTES.fetch_add(usize_to_u64(layout.size()), Ordering::Relaxed);
                REALLOC_NEW_BYTES.fetch_add(usize_to_u64(new_size), Ordering::Relaxed);
            }
        }

        new_ptr
    }
}

pub fn reset_allocation_counters() {
    ALLOC_CALLS.store(0, Ordering::Relaxed);
    ALLOC_ZEROED_CALLS.store(0, Ordering::Relaxed);
    ALLOC_FAILURES.store(0, Ordering::Relaxed);
    ALLOC_ZEROED_FAILURES.store(0, Ordering::Relaxed);
    ALLOC_BYTES.store(0, Ordering::Relaxed);
    ALLOC_ZEROED_BYTES.store(0, Ordering::Relaxed);
    DEALLOC_CALLS.store(0, Ordering::Relaxed);
    DEALLOC_BYTES.store(0, Ordering::Relaxed);
    REALLOC_CALLS.store(0, Ordering::Relaxed);
    REALLOC_FAILURES.store(0, Ordering::Relaxed);
    REALLOC_OLD_BYTES.store(0, Ordering::Relaxed);
    REALLOC_NEW_BYTES.store(0, Ordering::Relaxed);
}

pub fn allocation_snapshot() -> AllocationSnapshot {
    AllocationSnapshot {
        alloc_calls: ALLOC_CALLS.load(Ordering::Relaxed),
        alloc_zeroed_calls: ALLOC_ZEROED_CALLS.load(Ordering::Relaxed),
        alloc_failures: ALLOC_FAILURES.load(Ordering::Relaxed),
        alloc_zeroed_failures: ALLOC_ZEROED_FAILURES.load(Ordering::Relaxed),
        alloc_bytes: ALLOC_BYTES.load(Ordering::Relaxed),
        alloc_zeroed_bytes: ALLOC_ZEROED_BYTES.load(Ordering::Relaxed),
        dealloc_calls: DEALLOC_CALLS.load(Ordering::Relaxed),
        dealloc_bytes: DEALLOC_BYTES.load(Ordering::Relaxed),
        realloc_calls: REALLOC_CALLS.load(Ordering::Relaxed),
        realloc_failures: REALLOC_FAILURES.load(Ordering::Relaxed),
        realloc_old_bytes: REALLOC_OLD_BYTES.load(Ordering::Relaxed),
        realloc_new_bytes: REALLOC_NEW_BYTES.load(Ordering::Relaxed),
    }
}

static GLOBAL_PROFILER: LazyLock<Profiler> = LazyLock::new(Profiler::new);

#[inline]
pub fn global_profiler() -> &'static Profiler {
    &GLOBAL_PROFILER
}

#[macro_export]
macro_rules! profile {
    ($name:expr, $block:expr) => {{
        let name: &'static str = $name;
        let profiler = $crate::profiler::global_profiler();
        if profiler.is_enabled() {
            let _profile_guard = profiler.global_span(name);
            $block
        } else {
            $block
        }
    }};
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub entries: AtomicU64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_entries(&self, count: u64) {
        self.entries.store(count, Ordering::Relaxed);
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits.saturating_add(misses);

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.entries.store(0, Ordering::Relaxed);
    }
}

#[inline]
fn average_duration(duration: Duration, count: u64) -> Duration {
    if count == 0 {
        return Duration::ZERO;
    }

    let nanos = duration.as_nanos() / u128::from(count);
    Duration::from_nanos(nanos.min(u128::from(u64::MAX)) as u64)
}

#[inline]
fn duration_bucket(duration: Duration) -> usize {
    let mut upper_micros = 1u128;
    let micros = duration.as_micros();
    let mut bucket = 0usize;

    while bucket + 1 < PROFILE_HISTOGRAM_BUCKETS && micros > upper_micros {
        upper_micros <<= 1;
        bucket += 1;
    }

    bucket
}

#[inline]
fn bucket_upper_bound(bucket: usize) -> Duration {
    if bucket >= 63 {
        return Duration::MAX;
    }

    Duration::from_micros(1u64 << bucket)
}

#[inline]
fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[inline]
fn usize_to_u64(value: usize) -> u64 {
    value.try_into().unwrap_or(u64::MAX)
}

fn write_horizontal_rule(f: &mut Formatter<'_>, width: usize) -> FmtResult {
    for _ in 0..width {
        write!(f, "-")?;
    }
    writeln!(f)
}

#[cfg(test)]
mod tests {
    use std::alloc::{GlobalAlloc, Layout};
    use std::sync::{Mutex, MutexGuard};
    use std::time::Duration;

    use super::{
        CacheStats, Profiler, ProfilingAllocator, allocation_snapshot, global_profiler,
        reset_allocation_counters,
    };

    static PROFILER_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn profiler_test_lock() -> MutexGuard<'static, ()> {
        PROFILER_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn disabled_profiler_does_not_record() {
        let profiler = Profiler::new();

        profiler.record("scan", Duration::from_micros(10));

        assert!(profiler.get("scan").is_none());
    }

    #[test]
    fn records_duration_metrics_and_counters() {
        let _guard = profiler_test_lock();
        let profiler = Profiler::enabled();

        profiler.record("scan", Duration::from_micros(5));
        profiler.record_sample_enabled("scan", Duration::from_micros(20), Duration::from_micros(7));
        profiler.record_counter("files", 2);
        profiler.record_counter("files", 4);
        profiler.disable();

        let Some(metrics) = profiler.get("scan") else {
            panic!("profile entry should exist");
        };
        assert_eq!(metrics.count, 2);
        assert_eq!(metrics.total_duration, Duration::from_micros(25));
        assert_eq!(metrics.self_duration, Duration::from_micros(18));
        assert_eq!(metrics.child_duration, Duration::from_micros(7));

        let summary = profiler.summary();
        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].name, "scan");

        let counters = profiler.counter_summary();
        assert_eq!(counters.total("files"), 6);
    }

    #[test]
    fn global_profile_macro_records_spans() {
        let _guard = profiler_test_lock();
        let profiler = global_profiler();
        profiler.clear();
        profiler.enable();

        let value = crate::profile!("macro.scan", 42);

        profiler.disable();
        assert_eq!(value, 42);
        assert_eq!(profiler.get("macro.scan").map(|entry| entry.count), Some(1));
        profiler.clear();
    }

    #[test]
    fn profiling_allocator_records_allocation_pressure() {
        let _guard = profiler_test_lock();
        let profiler = global_profiler();
        profiler.clear();
        reset_allocation_counters();
        profiler.enable();

        let allocator = ProfilingAllocator::new();
        let Ok(layout) = Layout::from_size_align(32, 8) else {
            panic!("test layout should be valid");
        };

        // SAFETY: The layout is valid and the pointer is deallocated with the same allocator.
        let ptr = unsafe { GlobalAlloc::alloc(&allocator, layout) };
        assert!(!ptr.is_null());
        // SAFETY: The pointer came from the same allocator with the same layout.
        unsafe { GlobalAlloc::dealloc(&allocator, ptr, layout) };

        profiler.disable();
        let snapshot = allocation_snapshot();

        assert_eq!(snapshot.alloc_calls, 1);
        assert_eq!(snapshot.alloc_bytes, 32);
        assert_eq!(snapshot.dealloc_calls, 1);
        assert_eq!(snapshot.dealloc_bytes, 32);
    }

    #[test]
    fn cache_stats_track_hit_rate() {
        let stats = CacheStats::new();

        stats.hit();
        stats.miss();
        stats.set_entries(3);

        assert_eq!(stats.hit_rate(), 0.5);
        assert_eq!(stats.entries.load(std::sync::atomic::Ordering::Relaxed), 3);
    }
}
