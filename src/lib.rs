// Copyright (c) 2018-2019 Martin Larralde (martin.larralde@ens-paris-saclay.fr)
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
//! The [`Deblockator`] wraps another underlying allocator, and only uses it to
//! allocate large memory blocks. It maintains a linked-list of small
//! *heapblocks* which are constant-size memory blocks linked together
//! to emulate a growable heap. Heapblocks have a default size of `64kB`,
//! but various parameters can be defined at compile time using numerics
//! from the [`typenum`] crate.
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
//! The [`Deblockator`] can wraps non-global allocator, and needs a synchronisation
//! primitive to avoid race conditions. This is done using a *spinning mutex*
//! from the [`spin`] crate.
//!
//! # Usage
//!
//! ## Generic usage
//!
//! The provided [`Deblockator`] wraps any object implementing [`Alloc`]. For
//! instance, to use [`Deblockator`] with `jemalloc` to allocate the
//! heapblocks:
//! ```rust,no_run
//! #![feature(global_allocator)]
//! extern crate jemallocator;
//! extern crate deblockator;
//!
//! use jemallocator::Jemalloc;
//! use deblockator::Deblockator;
//!
//! #[global_allocator]
//! static GLOBAL: Deblockator<Jemalloc> = Deblockator::new(Jemalloc);
//! # fn main() {}
//! ```
//!
//! ## PS Vita target
//!
//! If you're compiling to PS Vita: use the [`Vitallocator`], which
//! wraps the `psp2` kernel API using [`psp2-sys`] bindings:
//!
//! ```rust,ignore
//! #![feature(global_allocator)]
//! extern crate deblockator;
//! extern crate vitallocator;
//!
//! use deblockator::Deblockator;
//! use vitallocator::Vitallocator;
//!
//! #[global_allocator]
//! static GLOBAL: Deblockator<Vitallocator> = Deblockator::new(Vitallocator::new());
//! # fn main() {}
//! ```
//!
//! [`spin`]: https://docs.rs/spin/
//! [`typenum`]: https://docs.rs/typenum/
//! [`Alloc`]: https://doc.rust-lang.org/nightly/std/alloc/trait.Alloc.html
//! [`Vitallocator`]: https://docs.rs/vitallocator/latest/vitallocator/struct.Vitallocator.html
//! [`KernelAllocator`]: struct.KernelAllocator.html

#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(const_mut_refs)]

#[cfg(test)]
use std as core;

extern crate spin;
extern crate typenum;

mod alloc;
mod hole;
mod utils;

// Public reexport of the generic allocator.
pub use alloc::Deblockator;
