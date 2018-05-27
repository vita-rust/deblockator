use core::alloc::Alloc;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::alloc::Opaque;
use core::cell::UnsafeCell;
use core::cmp::max;
use core::mem::align_of;
use core::mem::size_of;
use core::ptr::NonNull;

use super::hole::HeapBlock;
use super::hole::Hole;
use super::utils::align_up;
use super::Mutex;

pub const HEAP_BLOCK_SIZE: usize = 64 * 1024;
pub const HEAP_BLOCK_PADDING: usize = 4 * 1024;
pub const LARGE_OBJECT_SIZE: usize = 4096;

/// The PS Vita allocator.
///
/// Works by maintaining a non-continuous heap.
///
pub struct Allocator<A: Alloc> {
    mutex: Mutex<()>,
    block_allocator: UnsafeCell<A>,
    first_block: UnsafeCell<Option<&'static mut HeapBlock>>,
}

unsafe impl<A: Alloc> Sync for Allocator<A> {}
unsafe impl<A: Alloc> Send for Allocator<A> {}

impl<A: Alloc + Default> Default for Allocator<A> {
    fn default() -> Self {
        Self::new(A::default())
    }
}

impl<A: Alloc> Allocator<A> {
    /// Create a new kernel allocator.
    pub const fn new(alloc: A) -> Self {
        Allocator {
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

unsafe impl<A: Alloc> GlobalAlloc for Allocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut Opaque {
        let lock = self.mutex.lock();
        let allocator = &mut *self.block_allocator.get();

        // if the requested memory block is large, simply dedicate a single block
        if layout.size() >= LARGE_OBJECT_SIZE {
            return match allocator.alloc(self.padded(layout, HEAP_BLOCK_PADDING)) {
                Ok(ptr) => ptr.as_ptr() as *mut Opaque,
                Err(_) => ::core::ptr::null_mut::<u8>() as *mut Opaque,
            };
        }

        // Pad the layout to the minimum legal size
        let block_layout = {
            let mut size = max(HeapBlock::min_size(), layout.size());
            Layout::from_size_align_unchecked(align_up(size, align_of::<Hole>()), layout.align())
        };
        //
        // // traverse the heap blocks to find an allocatable block
        let mut next_block: *mut Option<&mut HeapBlock> = self.first_block.get();
        while let Some(ref mut block) = *next_block {
            if let Ok(ptr) = block.allocate_first_fit(block_layout) {
                return ptr.as_ptr() as *mut Opaque;
            };
            next_block = &mut block.next;
        }

        // No block can contain the requested layout: allocate a new one !
        let new_heap_layout = self.padded(layout, HEAP_BLOCK_PADDING);
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

        if layout.size() > LARGE_OBJECT_SIZE {
            let allocator = &mut *self.block_allocator.get();
            allocator.dealloc(
                NonNull::new(ptr).unwrap(),
                self.padded(layout, HEAP_BLOCK_PADDING),
            );
        } else {
            // panic!("TODO");
        }

        drop(lock)
    }
}
