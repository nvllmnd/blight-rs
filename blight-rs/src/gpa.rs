//! General Purpose Allocators, Most of which have no state and forward calls to mi_malloc_*/mi_calloc_* ect..
//!
//!
//!

use alloc::alloc::{AllocError, Allocator};

use crate::mimalloc::calloc_aligned;

#[derive(Debug, Clone, Copy, Default)]
pub struct MiMallocator;

unsafe impl Allocator for MiMallocator {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        let size = layout.size();
        let align = layout.align();
        calloc_aligned(1, size, align).ok_or(AllocError)
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        todo!()
    }
}
