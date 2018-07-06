extern crate jemallocator;
extern crate vitalloc;

use jemallocator::Jemalloc;
use vitalloc::Allocator;

#[global_allocator]
static GLOBAL: Allocator<Jemalloc> = Allocator::new(Jemalloc);
