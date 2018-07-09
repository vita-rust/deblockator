#![feature(alloc_system)]

extern crate alloc_system;
extern crate vitalloc;

use alloc_system::System;
use vitalloc::Vitalloc;

mod cases;

#[global_allocator]
static GLOBAL: Vitalloc<System> = Vitalloc::new(System);

#[test]
fn test_small_alloc() {
    cases::small_alloc();
}
