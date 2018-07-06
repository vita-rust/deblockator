#![feature(alloc_system)]

extern crate alloc_system;
extern crate vitalloc;

use alloc_system::System;
use vitalloc::Allocator;

#[global_allocator]
static GLOBAL: Allocator<System> = Allocator::new(System);
