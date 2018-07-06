// Copyright (c) 2018 Martin Larralde (martin.larralde@ens-paris-saclay.fr)
//
// Licensed under MIT license (the COPYING file). This file may not be
// copied, modified, or distributed except according to those terms.

//! An allocator using a growable heap made of smaller linked memory blocks.
//!
//! Designed to work as a wrapper for more-limited fixed-size allocators, such
//! as the ones found in embedded systems.
//!
//! # Introduction
//!
//! This crate was designed first as an allocator for the PS Vita game console,
//! which provides a very limited memory allocation API: its allocator will only
//! allocate memory blocks of `4kB`-aligned memory. As such, it cannot be used
//! as a performant system allocator for smaller objects.
//!
//! # Algorithm
//!
//! The [`Vitalloc`] wraps another underlying allocator, and only uses it to
//! allocate large memory blocks. It maintains a linked-list of small
//! *heapblocks* which are constant-size memory blocks linked together
//! to emulate a growable heap. Heapblocks have a default size of `64kB`.
//!
//! ## Allocation
//!
//! When a request is made to allocate memory, the allocator will iterate
//! through all the heapblocks, using a **first-fit** allocation method to try
//! to find an appropriate free memory location. If no heapblock can fit the
//! requested layout, then a new heapblock is allocated.
//!
//! Allocation of very large layouts (more than `16kB`) are done using the
//! underlying allocator directly. This avoids the possible case of memory
//! retention with small blocks preventing the deallocation of a very large
//! block, were the small block to outlive the larger one.
//!
//! ## Deallocation
//!
//! If the allocated layout size is larger than the large layout limit, we
//! simply transmit the deallocation request to the underlying allocator.
//! Otherwise, we traverse the heapblocks to find the one the memory block
//! belongs to. A heapblock is deallocated when it is completely empty.
//!
//! ## Synchronisation
//!
//! The [`Vitalloc`] can wraps non-global allocator, and needs a synchronisation
//! primitive to avoid race conditions. This is done using a *spinning mutex*
//! from the [`spin`] crate.
//!
//! # Usage
//!
//! ## Generic usage
//!
//! The provided [`Vitalloc`] wraps any object implementing [`Alloc`]. For
//! instance, to use [`Vitalloc`] with the `system_allocator` to allocate the
//! heapblocks:
//! ```rust,no_run
//! #![feature(global_allocator, alloc_system)]
//! extern crate alloc_system;
//! extern crate vitalloc;
//!
//! use alloc_system::System;
//! use vitalloc::Vitalloc;
//!
//! #[global_allocator]
//! static GLOBAL: Vitalloc<System> = Vitalloc::new(System);
//! # fn main() {}
//! ```
//!
//! ## PS Vita target
//!
//! If you're compiling to PS Vita: use the included [`KernelVitalloc`], which
//! wraps the `psp2` kernel API using [`psp2-sys`] bindings:
//!
//! ```rust,no_run
//! #![feature(global_allocator)]
//! extern crate vitalloc;
//! use vitalloc::{Vitalloc, KernelAllocator};
//!
//! #[global_allocator]
//! static GLOBAL: Vitalloc<KernelAllocator> = Vitalloc::new(KernelAllocator::new());
//! # fn main() {}
//! ```

#![cfg_attr(not(test), no_std)]
#![feature(alloc)]
#![feature(allocator_api)]
#![feature(const_fn)]
#![feature(doc_cfg)]
#![feature(trait_alias)]

#[cfg(test)]
use std as core;

#[macro_use]
extern crate cfg_if;
extern crate spin;
extern crate typenum;

mod alloc;
mod hole;
mod utils;

// Public reexport of the generic allocator.
pub use alloc::Vitalloc;

// Feature compilation of the kernel allocator
cfg_if! {
    if #[cfg(feature = "kernel-allocator")] {
        extern crate psp2_sys;
        mod kernel;
        pub use kernel::KernelAllocator;
    }
}
