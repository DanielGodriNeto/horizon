//! Wall-time counters for the worldgen's inner stages, read by `cpu_bench`.
//!
//! `spans.rs` lives in the client, which the world crate cannot reach, so the
//! timers live here. Without the `prof` feature `scope` is an empty guard that
//! the compiler removes; with it each scope adds its elapsed nanoseconds to a
//! process-wide counter. Scopes must wrap whole calls (per chunk, not per
//! block): a clock read costs about as much as a cheap block.

/// The slots, in the order `take` returns them. Slices of one chunk's proto
/// stage; `SampleVolume` nests inside `FillNoise`.
#[derive(Clone, Copy)]
pub enum Slot {
    FillNoise,
    SampleVolume,
    BuildSurface,
}

pub const NAMES: [&str; 3] = ["fill_noise", "sample_volume", "build_surface"];

#[cfg(feature = "prof")]
mod on {
    use super::Slot;
    use std::{
        sync::atomic::{AtomicU64, Ordering::Relaxed},
        time::Instant,
    };

    static NANOS: [AtomicU64; super::NAMES.len()] = [const { AtomicU64::new(0) }; super::NAMES.len()];

    pub struct Scope(Slot, Instant);

    pub fn scope(slot: Slot) -> Scope {
        Scope(slot, Instant::now())
    }

    impl Drop for Scope {
        fn drop(&mut self) {
            NANOS[self.0 as usize].fetch_add(self.1.elapsed().as_nanos() as u64, Relaxed);
        }
    }

    /// Reads and zeroes every counter.
    pub fn take() -> [u64; super::NAMES.len()] {
        std::array::from_fn(|i| NANOS[i].swap(0, Relaxed))
    }
}

#[cfg(not(feature = "prof"))]
mod on {
    use super::Slot;

    pub struct Scope;

    pub fn scope(_: Slot) -> Scope {
        Scope
    }

    pub fn take() -> [u64; super::NAMES.len()] {
        [0; super::NAMES.len()]
    }
}

pub use on::{Scope, scope, take};
