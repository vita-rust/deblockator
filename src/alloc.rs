use core::alloc::Alloc;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::cell::UnsafeCell;
use core::cmp::max;
use core::marker::PhantomData;
use core::mem::align_of;
use core::ptr::NonNull;

use spin::Mutex;
use typenum::consts::U16384;
use typenum::consts::U32768;
use typenum::consts::U4096;
use typenum::consts::U65536;
use typenum::PowerOfTwo;
use typenum::Unsigned;

use super::hole::HeapBlock;
use super::hole::Hole;
use super::utils::align_up;

#[cfg(not(test))]
/// A generic allocator using a linked heap made of smaller blocks.
///
/// Horizontal heap-growth allows to emulate a vertically-infinite heap using
/// independent memory blocks linked together as a linked list. This allows to
/// circumvent the PS Vita kernel limitations of allocating `4kB`-aligned memory
/// by creating a virtually growable heap, and using a plain heap allocator on it.
///
/// This struct internals were adapted from [`linked-list-allocator`], although they
/// do not share the same data layouts and synchronisation mechanisms.
///
/// # Compile-time configuration
///
/// Allocation parameters can be changed at compile time using numeric types
/// from the [`typenum`](https://docs.rs/typenum) crate. The parameters are
/// defined (in the order of appearance in the struct signature):
///
/// * **`BS`** (block size): the size of a single heap block.
/// * **`BA`** (block alignment): the alignment required for a heap block.
/// * **`LS`** (large block size): the size above which an individual block is
///   allocated instead of using heap blocks. A typical value is 1/4th of the
///   block size. *Undefined behaviour if not lower than the block size !*
/// * **`LA`** (large block alignment): the alignment required for a large block.
///
/// [`linked-list-allocator`]: https://crates.io/crates/linked-list-allocator
pub struct Vitalloc<A, BS = U65536, BA = U4096, LS = U16384, LA = U4096>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
    __block_size: PhantomData<BS>,
    __block_padding: PhantomData<BA>,
    __large_size: PhantomData<LS>,
    __large_padding: PhantomData<LA>,
    mutex: Mutex<()>,
    block_allocator: UnsafeCell<A>,
    first_block: UnsafeCell<Option<&'static mut HeapBlock>>,
}

#[cfg(test)]
/// Test definition with public variables.
pub struct Vitalloc<A, BS = U65536, BA = U4096, LS = U16384, LA = U4096>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
    __block_size: PhantomData<BS>,
    __block_padding: PhantomData<BA>,
    __large_size: PhantomData<LS>,
    __large_padding: PhantomData<LA>,
    pub mutex: Mutex<()>,
    pub block_allocator: UnsafeCell<A>,
    pub first_block: UnsafeCell<Option<&'static mut HeapBlock>>,
}

unsafe impl<A, BS, BA, LS, LA> Sync for Vitalloc<A, BS, BA, LS, LA>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
}

unsafe impl<A, BS, BA, LS, LA> Send for Vitalloc<A, BS, BA, LS, LA>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
}

impl<A, BS, BA, LS, LA> Default for Vitalloc<A, BS, BA, LS, LA>
where
    A: Alloc + Default,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
    fn default() -> Self {
        Self::new(A::default())
    }
}

impl<A, BS, BA, LS, LA> Vitalloc<A, BS, BA, LS, LA>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
    /// Create a new allocator instance, wrapping the given allocator.
    pub const fn new(alloc: A) -> Self {
        Vitalloc {
            __block_size: PhantomData,
            __block_padding: PhantomData,
            __large_size: PhantomData,
            __large_padding: PhantomData,
            mutex: Mutex::new(()),
            block_allocator: UnsafeCell::new(alloc),
            first_block: UnsafeCell::new(None),
        }
    }

    /// Create a kernel-compatible layout that can fit the requested layout
    unsafe fn padded(&self, layout: Layout, align: usize) -> Layout {
        let padding = layout.padding_needed_for(align);
        Layout::from_size_align_unchecked(layout.size() + padding, align)
    }
}

unsafe impl<A, BS, BA, LS, LA> GlobalAlloc for Vitalloc<A, BS, BA, LS, LA>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let lock = self.mutex.lock();
        let allocator = &mut *self.block_allocator.get();

        // if the requested memory block is large, simply dedicate a single block
        if layout.size() >= LS::to_usize() {
            return match allocator.alloc(self.padded(layout, LA::to_usize())) {
                Ok(ptr) => ptr.as_ptr() as *mut u8,
                Err(_) => ::core::ptr::null_mut::<u8>(),
            };
        }

        // Pad the layout to the minimum legal size
        let block_layout = {
            let mut size = max(HeapBlock::<BS>::min_size(), layout.size());
            Layout::from_size_align_unchecked(align_up(size, align_of::<Hole>()), layout.align())
        };

        // traverse the heap blocks to find an allocatable block
        let mut next_block: *mut Option<&mut HeapBlock> = self.first_block.get();
        while let Some(ref mut block) = *next_block {
            if let Ok(ptr) = block.allocate_first_fit(block_layout) {
                return ptr.as_ptr() as *mut u8;
            };
            next_block = &mut block.next;
        }

        // No block can contain the requested layout: allocate a new one !
        let new_heap_layout = Layout::from_size_align_unchecked(BS::to_usize(), BA::to_usize());
        let new_heap_ptr = match allocator.alloc(new_heap_layout) {
            Ok(ptr) => ptr.as_ptr() as *mut u8,
            Err(_) => return ::core::ptr::null_mut::<u8>(),
            // Err(_) => return 0xDEADBEEF as usize as *mut _,
        };

        // Initialize the block and use it to allocate
        let new_block = HeapBlock::<BS>::new(new_heap_ptr as usize);
        let new_block_ptr = match new_block.allocate_first_fit(block_layout) {
            Ok(mem) => mem.as_ptr() as *mut _,
            Err(_) => return ::core::ptr::null_mut::<u8>(),
            // Err(_) => return 0xCAFEBABE as usize as *mut _,
        };
        *next_block = Some(new_block);

        drop(lock);
        new_block_ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let lock = self.mutex.lock();

        if layout.size() > LS::to_usize() {
            let allocator = &mut *self.block_allocator.get();
            allocator.dealloc(
                NonNull::new(ptr).unwrap(),
                self.padded(layout, LA::to_usize()),
            );
        } else {
            // TODO
        }

        drop(lock)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use core::alloc::AllocErr;
    use core::mem::size_of;

    use typenum::consts::U2048;

    struct MockAlloc {
        pub allocated: [bool; 3],
        pub blocks: [[u8; 4096]; 3],
    }

    impl MockAlloc {
        pub fn new() -> Self {
            Self {
                allocated: [false; 3],
                blocks: [[0; 4096], [0; 4096], [0; 4096]],
            }
        }
    }

    unsafe impl Alloc for MockAlloc {
        unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
            for i in 0..self.blocks.len() {
                if !self.allocated[i] {
                    self.allocated[i] = true;
                    return NonNull::new(self.blocks[i].as_mut().as_mut_ptr()).ok_or(AllocErr);
                }
            }
            Err(AllocErr)
        }

        unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
            for i in 0..self.blocks.len() {
                if ptr.as_ptr() == self.blocks[i].as_mut().as_mut_ptr() {
                    if !self.allocated[i] {
                        panic!("double free")
                    } else {
                        self.allocated[i] = false;
                        return;
                    }
                }
            }
            panic!("no such block !")
        }
    }

    #[test]
    /// Test the mock allocator works as expected.
    fn mockalloc() {
        unsafe {
            let mut ma = MockAlloc::new();
            let layout = Layout::from_size_align_unchecked(4096, 4096);

            let pt1 = ma.alloc(layout).expect("could not allocate block 1");
            let pt2 = ma.alloc(layout).expect("could not allocate block 2");
            let pt3 = ma.alloc(layout).expect("could not allocate block 3");
            ma.alloc(layout).expect_err("all blocks were not allocated");

            for i in 0..3 {
                assert!(ma.allocated[i]);
            }

            ma.dealloc(pt1, layout);
            assert!(!ma.allocated[0]);

            ma.dealloc(pt3, layout);
            assert!(!ma.allocated[2]);

            let pt4 = ma.alloc(layout).expect("could not allocate block 4");
            assert!(ma.allocated[0]);
            assert!(!ma.allocated[2]);
            assert_eq!(pt4.as_ptr(), pt1.as_ptr());
        }
    }

    #[test]
    /// Check the underlying blocks are allocated as expected.
    fn vitalloc_blocks() {
        let ma = MockAlloc::new();
        let va: Vitalloc<MockAlloc, U4096, U4096, U2048, U4096> = Vitalloc::new(ma);

        unsafe {
            // quick accessor to the allocated blocks
            let allocated = || va.block_allocator.get().read().allocated;
            let blocks = || va.block_allocator.get().read().blocks;

            // Allocate a single boxed u32
            let layout = Layout::from_size_align(32, 8).expect("bad layout");
            let ptr1 = NonNull::new(va.alloc(layout)).expect("could not allocate 1");
            ::core::ptr::write(ptr1.as_ptr(), 255);
            assert_eq!(allocated(), [true, false, false]);

            // Allocate a second boxed u32
            let ptr2 = NonNull::new(va.alloc(layout)).expect("could not allocate 2");
            ::core::ptr::write(ptr2.as_ptr(), 254);
            assert_eq!(allocated(), [true, false, false]);

            // Allocate a large object to the second block
            let layout = Layout::from_size_align(3129, 4096).expect("bad layout");
            let ptr3 = NonNull::new(va.alloc(layout)).expect("could not allocate 3");
            assert_eq!(allocated(), [true, true, false]);

            // Deallocate the first u32
            let layout = Layout::from_size_align(32, 8).expect("bad layout");
            va.dealloc(ptr1.as_ptr(), layout);

            // FIXME: Reallocate the first u32 (hopefully at the same place)
            // let ptr4 = NonNull::new(va.alloc(layout)).expect("could not allocate 4");
            // assert_eq!(ptr4.as_ptr(), ptr1.as_ptr());

            // Deallocate the large block
            let layout = Layout::from_size_align(3129, 4096).expect("bad layout");
            va.dealloc(ptr3.as_ptr(), layout);
            assert_eq!(allocated(), [true, false, false]);
        }
    }

}
