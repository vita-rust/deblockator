// Copyright (c) 2018 Martin Larralde (martin.larralde@ens-cachan.fr)
// See the COPYING file at the top-level directory of this repository.
//
// Licensed under MIT license (the LICENSE-MIT file). This file may not be
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
//! The [`Allocator`] wraps another underlying allocator, and only uses it to
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
//! # Usage
//!
//! If you're compiling to PS Vita: use the included [`KernelAllocator`], which
//! wraps the `psp2` kernel API:
//!
//! ```rust,ignore
//! #![feature(global_allocator)]
//! extern crate vitalloc;
//!
//! #[global_allocator]
//! static ALLOC: vitalloc::Allocator =
//!     vitalloc::Vitalloc::new(vitalloc::KernelAllocator::new());
//! ```

#![feature(alloc)]
#![feature(allocator_api)]
#![feature(const_fn)]
#![feature(doc_cfg)]
#![feature(trait_alias)]
#![cfg_attr(not(test), no_std)]

extern crate typenum;

mod alloc;
mod hole;
mod utils;

#[cfg(test)]
use std as core;

// Public reexport of the generic allocator.
pub use alloc::Allocator;

// PS Vita specific exports
#[cfg(target_os = "vita")]
extern crate psp2_sys;
#[cfg(target_os = "vita")]
mod kernel;
#[cfg(target_os = "vita")]
mod mutex;
#[cfg(any(target_os = "vita", feature = "doc"))]
pub use kernel::KernelAllocator;
#[cfg(target_os = "vita")]
pub use mutex::{Mutex, MutexGuard};

// Other targets exports (mostly for testing)
#[cfg(not(target_os = "vita"))]
extern crate spin;
#[cfg(not(target_os = "vita"))]
pub use spin::{Mutex, MutexGuard};
