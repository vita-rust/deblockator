extern crate jemallocator;
extern crate deblockator;

use jemallocator::Jemalloc;
use deblockator::Deblockator;

mod cases;

#[global_allocator]
static GLOBAL: Deblockator<Jemalloc> = Deblockator::new(Jemalloc);

#[test]
fn test_small_alloc() {
    cases::small_alloc();
}
