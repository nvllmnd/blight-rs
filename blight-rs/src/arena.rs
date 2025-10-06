//!
//! Contains an [Arena] type that implements a simple arena allocator,
//! that uses a linked list of memory blocks to expand capacity dynamically as needed
//!
//!

use core::{
    alloc::Layout,
    cell::{Cell, UnsafeCell},
    mem::MaybeUninit,
    ptr::{self, NonNull},
};

use alloc::{alloc::Allocator, boxed::Box, vec::Vec};

use crate::heap::{alloc::Heap, handle::HeapHandle};
#[repr(C)]
#[derive(Debug)]
struct ArenaBlock {
    prev: Option<Box<Self>>,
    used: Cell<i32>,
    storage: UnsafeCell<[u8]>,
}

impl ArenaBlock {
    /// Creates a new `ArenaBlock` with a storage section of `storage_size` bytes.
    fn new(storage_size: usize, heap: &Heap) -> Box<Self, HeapHandle<'_>> {
        let layout = Layout::from_size_align(
            size_of::<Option<Box<Self>>>() + size_of::<Cell<i32>>() + storage_size,
            8,
        )
        .unwrap();

        unsafe {
            // 2. Allocate the memory.
            let raw_ptr = heap.allocate(layout).unwrap().as_ptr() as *mut ArenaBlock;

            // 3. Initialize the fields of the struct.
            // We use `ptr::write` to write to the uninitialized memory.
            ptr::write(&mut (*raw_ptr).prev, None);
            ptr::write(&mut (*raw_ptr).used, Cell::new(0));
            let x = (*raw_ptr).storage.get().len();

            // The `storage` field itself is a "fat pointer" containing a pointer
            // to the data and the length. We need to create that fat pointer.
            // The pointer to the data is the location right after the `used` field.
            // However, we are creating a Box<ArenaBlock> which is already a fat pointer
            // to our DST. The length is part of the pointer.

            // To properly create the `Box<DST>`, we first create a raw pointer to a slice,
            // which is a fat pointer.
            let slice_ptr: *mut [u8] =
                ptr::slice_from_raw_parts_mut((*raw_ptr).storage.get() as *mut u8, storage_size);

            // Then we can create a pointer to our DST struct from the data pointer and the length.
            let dst_ptr: *mut Self = ptr::from_raw_parts_mut(raw_ptr as *mut (), storage_size);

            assert!(storage_size == (*dst_ptr).storage.get().len());
            // 4. Create a `Box` from the raw pointer.
            // This transfers ownership of the allocated memory to the `Box`.
            Box::from_raw_in(dst_ptr, heap.as_handle())
        }
    }

    /// Returns a mutable slice to the storage.
    fn storage(&mut self) -> &mut [u8] {
        unsafe {
            // The fat pointer for `Box<Self>` knows the length of the `storage` slice.
            let ptr = self.storage.get();
            &mut *ptr
        }
    }
}

// #[repr(C)]
// #[derive(Debug)]
// struct ArenaBlock<T: ?Sized> {
//     prev: Option<Box<Self>>,
//     used: Cell<i32>,
//
//     storage: UnsafeCell<T>,
// }

#[cfg(test)]
mod tests {
    use crate::{arena::ArenaBlock, heap::alloc::Heap};

    #[test]
    fn allocate_dst() {
        let h = Heap::default();
        let a = ArenaBlock::new(255, &h);
        // assert!(a.storage.get().len() > 0);
    }
}

// #[derive(Debug)]
// pub struct Arena {
//     heap: Heap,
//     mem: Option<Box<ArenaBlock<[u8]>>>,
// }
//
// impl Arena {
//     pub fn new() -> Self {
//         Self {
//             heap: Heap::default(),
//             mem: None,
//         }
//     }
//
//     pub fn with_init_block(size: usize) -> Self {
//         const HEAD_SIZE: usize = size_of::<MaybeUninit<ArenaBlock<[u8; 0]>>>();
//         let layout_size = HEAD_SIZE + size;
//         let s = Self::new();
//         let layout = Layout::from_size_align(layout_size, align_of::<ArenaBlock<[u8; 0]>>())
//             .expect("Failed to create layout for ArenaBlock");
//
//         let b = s
//             .heap
//             .allocate(layout)
//             .expect("Failed to allocate new Arena block!")
//             .cast::<u8>()
//
//         // let b = unsafe { Box::from_raw_in(b.as_ptr() as *mut ArenaBlock<[u8]>, s.heap.as_handle()) };
//     }
// }
//
// impl Default for Arena {
//     fn default() -> Self {
//         Self::new()
//     }
// }
