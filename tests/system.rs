#![feature(alloc_system)]

extern crate alloc_system;
extern crate vitalloc;

use alloc_system::System;
use vitalloc::Vitalloc;

#[global_allocator]
static GLOBAL: Vitalloc<System> = Vitalloc::new(System);
