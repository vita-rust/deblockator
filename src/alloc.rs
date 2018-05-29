use core::alloc::Alloc;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::alloc::Opaque;
use core::cell::UnsafeCell;
use core::cmp::max;
use core::marker::PhantomData;
use core::mem::align_of;
use core::ptr::NonNull;

use typenum::consts::U32768;
use typenum::consts::U4096;
use typenum::consts::U65536;
use typenum::Less;
use typenum::PowerOfTwo;
use typenum::Unsigned;

use super::hole::HeapBlock;
use super::hole::Hole;
use super::utils::align_up;
use super::Mutex;

/// A generic allocator using a linked heap, designed for the PS Vita.
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
/// * **BS** (block size): the size of a single heap block.
/// * **BA** (block alignment): the alignment required for a heap block.
/// * **LS** (large block size): the size above which an individual block is
///   allocated instead of using heap blocks.
//    *Must be lower than the block size !*
/// * **LA** (large block alignment): the alignment required for a large block.
///
/// # Usage
///
///
///
/// [`linked-list-allocator`]: https://crates.io/crates/linked-list-allocator
pub struct Allocator<A, BS = U65536, BA = U4096, LS = U32768, LA = U4096>
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

unsafe impl<A, BS, BA, LS, LA> Sync for Allocator<A, BS, BA, LS, LA>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
}

unsafe impl<A, BS, BA, LS, LA> Send for Allocator<A, BS, BA, LS, LA>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
}

impl<A, BS, BA, LS, LA> Default for Allocator<A, BS, BA, LS, LA>
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

impl<A, BS, BA, LS, LA> Allocator<A, BS, BA, LS, LA>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
    /// Create a new allocator instance, wrapping the given allocator.
    pub const fn new(alloc: A) -> Self {
        Allocator {
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

unsafe impl<A, BS, BA, LS, LA> GlobalAlloc for Allocator<A, BS, BA, LS, LA>
where
    A: Alloc,
    BS: Unsigned,
    BA: Unsigned + PowerOfTwo,
    LS: Unsigned,
    LA: Unsigned + PowerOfTwo,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut Opaque {
        let lock = self.mutex.lock();
        let allocator = &mut *self.block_allocator.get();

        // if the requested memory block is large, simply dedicate a single block
        if layout.size() >= LS::to_usize() {
            return match allocator.alloc(self.padded(layout, LA::to_usize())) {
                Ok(ptr) => ptr.as_ptr() as *mut Opaque,
                Err(_) => ::core::ptr::null_mut::<u8>() as *mut Opaque,
            };
        }

        // Pad the layout to the minimum legal size
        let block_layout = {
            let mut size = max(HeapBlock::min_size(), layout.size());
            Layout::from_size_align_unchecked(align_up(size, align_of::<Hole>()), layout.align())
        };

        // traverse the heap blocks to find an allocatable block
        let mut next_block: *mut Option<&mut HeapBlock> = self.first_block.get();
        while let Some(ref mut block) = *next_block {
            if let Ok(ptr) = block.allocate_first_fit(block_layout) {
                return ptr.as_ptr() as *mut Opaque;
            };
            next_block = &mut block.next;
        }

        // No block can contain the requested layout: allocate a new one !
        let new_heap_layout = Layout::from_size_align_unchecked(BS::to_usize(), BA::to_usize());
        let new_heap_ptr = match allocator.alloc(new_heap_layout) {
            Ok(ptr) => ptr.as_ptr() as *mut Opaque,
            Err(_) => return ::core::ptr::null_mut::<*mut Opaque>() as *mut Opaque,
            // Err(_) => return 0xDEADBEEF as usize as *mut _,
        };

        // Initialize the block and use it to allocate
        let new_block = HeapBlock::new(new_heap_ptr as usize, new_heap_layout.size());
        let new_block_ptr = match new_block.allocate_first_fit(block_layout) {
            Ok(mem) => mem.as_ptr() as *mut _,
            Err(_) => return ::core::ptr::null_mut::<*mut Opaque>() as *mut Opaque,
            // Err(_) => return 0xCAFEBABE as usize as *mut _,
        };
        *next_block = Some(new_block);

        drop(lock);
        new_block_ptr as *mut Opaque
    }

    unsafe fn dealloc(&self, ptr: *mut Opaque, layout: Layout) {
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
