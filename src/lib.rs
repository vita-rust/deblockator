// Copyright (c) 2018 Martin Larralde (martin.larralde@ens-cachan.fr)
// See the COPYING file at the top-level directory of this repository.
//
// Licensed under MIT license (the LICENSE-MIT file). This file may not be
// copied, modified, or distributed except according to those terms.

//! An allocator using a growable heap made of smaller linked memory blocks.
//!
//!

#![feature(alloc, allocator_api, const_fn)]
#![crate_name = "vitalloc"]
#![crate_type = "staticlib"]
#![cfg_attr(not(test), no_std)]

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
#[cfg(target_os = "vita")]
pub use kernel::KernelAllocator;
#[cfg(target_os = "vita")]
pub use mutex::{Mutex, MutexGuard};

// Other targets exports (mostly for testing)
#[cfg(not(target_os = "vita"))]
extern crate spin;
#[cfg(not(target_os = "vita"))]
pub use spin::{Mutex, MutexGuard};
