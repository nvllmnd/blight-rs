//! General Purpose Allocator which wraps mimalloc calls and
//! defines the GlobalAlloc and Allocator traits
//!

use core::{alloc::GlobalAlloc, ffi::c_void, ptr::NonNull};

use alloc::alloc::{AllocError, Allocator, handle_alloc_error};
use mimalloc_bindgen::api::mi_free;

use crate::mimalloc::{calloc_aligned, malloc_aligned, mifree, realloc, recalloc, recalloc_aligned};

#[derive(Debug, Clone, Copy, Default)]
pub struct MiMallocator;

unsafe impl Allocator for MiMallocator {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        let size = layout.size();
        let align = layout.align();
        if size > 0 {
            calloc_aligned(1, size, align).ok_or(AllocError)
        } else {
            let p = NonNull::slice_from_raw_parts(NonNull::dangling(), 0);
            Ok(p)
        }
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        if layout.size() > 0 {
            unsafe {
                mifree(ptr);
            }
        }
    }

    fn allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, AllocError> {
        self.allocate(layout)
    }

    unsafe fn grow(
        &self,
        ptr: core::ptr::NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, AllocError> {
        core::debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
        );

        recalloc_aligned(ptr, 1, new_layout.size(), new_layout.align()).ok_or(AllocError)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: core::ptr::NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, AllocError> {
        core::debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
        );

        unsafe { self.grow(ptr, old_layout, new_layout) }
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        core::debug_assert!(
            new_layout.size() <= old_layout.size(),
            "`new_layout.size()` must be smaller than or equal to `old_layout.size()`"
        );
        recalloc_aligned(ptr, 1, new_layout.size(), new_layout.align()).ok_or(AllocError)
    }
}

unsafe impl GlobalAlloc for MiMallocator
where
    Self: Allocator,
{
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if let Ok(ptr) = self.allocate(layout) {
            ptr.cast::<u8>().as_ptr()
        } else {
            handle_alloc_error(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if let Some(ptr) = NonNull::new(ptr) {
            unsafe { self.deallocate(ptr, layout) }
        }
    }

    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        unsafe { self.alloc(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: core::alloc::Layout, new_size: usize) -> *mut u8 {
        if let Some(ptr) = NonNull::new(ptr) {
            if let Some(ptr) = recalloc_aligned(ptr, 1, new_size, layout.align()) {
                ptr.cast::<u8>().as_ptr()
            } else {
                handle_alloc_error(layout)
            }
        } else {
            handle_alloc_error(layout)
        }
    }
}
