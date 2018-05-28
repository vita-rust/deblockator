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

// Other targets exports
#[cfg(not(target_os = "vita"))]
extern crate spin;
#[cfg(not(target_os = "vita"))]
pub use spin::{Mutex, MutexGuard};
