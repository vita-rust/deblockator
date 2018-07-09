extern crate jemallocator;
extern crate vitalloc;

use jemallocator::Jemalloc;
use vitalloc::Vitalloc;

mod cases;

#[global_allocator]
static GLOBAL: Vitalloc<Jemalloc> = Vitalloc::new(Jemalloc);

#[test]
fn test_small_alloc() {
    cases::small_alloc();
}
