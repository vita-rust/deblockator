extern crate jemallocator;
extern crate vitalloc;

use jemallocator::Jemalloc;
use vitalloc::Vitalloc;

#[global_allocator]
static GLOBAL: Vitalloc<Jemalloc> = Vitalloc::new(Jemalloc);
