#![feature(alloc, allocator_api, const_fn)]
#![no_std]
#![crate_name = "vitalloc"]
#![crate_type = "staticlib"]

extern crate psp2_sys;

#[cfg(target_os = "vita")]
mod utils;

#[cfg(target_os = "vita")]
pub mod mutex;
#[cfg(target_os = "vita")]
pub use mutex::{Mutex, MutexGuard};

#[cfg(not(target_os = "vita"))]
extern crate spin;
#[cfg(not(target_os = "vita"))]
pub use spin::{Mutex, MutexGuard};
