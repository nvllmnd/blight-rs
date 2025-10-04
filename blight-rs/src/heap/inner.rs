use core::{ffi::c_void, ptr::NonNull};

use anyhow::{Context, ensure};
use mimalloc_bindgen::api::{
    mi_heap_calloc, mi_heap_calloc_aligned, mi_heap_malloc, mi_heap_malloc_aligned,
    mi_heap_malloc_small, mi_heap_realloc, mi_heap_realloc_aligned, mi_heap_recalloc,
    mi_heap_recalloc_aligned, mi_heap_t,
};

use crate::{
    heap::{InnerHeap, alloc::Heap},
    mimalloc::MIMALLOC_SMALL_SIZE_MAX,
};
//
// #[repr(transparent)]
// #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct InnerHeap(pub MiHeapPtr);
impl InnerHeap {
    pub const fn handle(&self) -> NonNull<mi_heap_t> {
        self.0
    }

    pub const fn handle_ptr(&self) -> *mut mi_heap_t {
        self.handle().as_ptr()
    }

    pub fn malloc(&self, size: usize) -> Option<NonNull<[u8]>> {
        let p = unsafe { mi_heap_malloc(self.handle_ptr(), size) }.cast::<u8>();
        let p = NonNull::new(p)?;
        let sl = NonNull::slice_from_raw_parts(p, size);
        Some(sl)
    }

    pub fn malloc_aligned(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        assert!(align.is_power_of_two());

        let p = unsafe { mi_heap_malloc_aligned(self.handle_ptr(), size, align) }.cast::<u8>();
        let p = NonNull::new(p)?;
        let sl = NonNull::slice_from_raw_parts(p, size);
        Some(sl)
    }

    pub fn malloc_small(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.try_malloc_small(size).ok()
    }

    pub fn try_malloc_small(&self, size: usize) -> anyhow::Result<NonNull<[u8]>> {
        ensure!(
            size <= MIMALLOC_SMALL_SIZE_MAX,
            "size must be <= MIMALLOC_SMALL_SIZE_MAX"
        );
        let p = unsafe { mi_heap_malloc_small(self.handle_ptr(), size) }.cast::<u8>();
        let p = NonNull::new(p).context("mi_heap_malloc_small returned nullptr!")?;
        let p = NonNull::slice_from_raw_parts(p, size);
        Ok(p)
    }

    pub fn calloc(&self, count: usize, size: usize) -> Option<NonNull<[u8]>> {
        let p = unsafe { mi_heap_calloc(self.handle_ptr(), count, size) }.cast::<u8>();
        let p = NonNull::new(p)?;
        let sl = NonNull::slice_from_raw_parts(p, size);
        Some(sl)
    }

    pub fn calloc_aligned(&self, count: usize, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        assert!(align.is_power_of_two());
        let p = unsafe { mi_heap_calloc_aligned(self.handle_ptr(), count, size, align) }.cast::<u8>();
        let p = NonNull::new(p)?;
        let sl = NonNull::slice_from_raw_parts(p, size);
        Some(sl)
    }

    pub fn recalloc(&self, ptr: NonNull<u8>, new_count: usize, size: usize) -> Option<NonNull<[u8]>> {
        let p = unsafe {
            mi_heap_recalloc(self.handle_ptr(), ptr.cast::<c_void>().as_ptr(), new_count, size)
        }
        .cast::<u8>();
        let p = NonNull::new(p)?;
        let sl = NonNull::slice_from_raw_parts(p, new_count * size);
        Some(sl)
    }

    pub fn recalloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_count: usize,
        size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        let p = unsafe {
            mi_heap_recalloc_aligned(
                self.handle_ptr(),
                ptr.cast::<c_void>().as_ptr(),
                new_count,
                size,
                align,
            )
        }
        .cast::<u8>();
        let p = NonNull::new(p)?;
        let sl = NonNull::slice_from_raw_parts(p, new_count * size);
        Some(sl)
    }

    pub fn realloc(&self, ptr: NonNull<u8>, new_size: usize) -> Option<NonNull<[u8]>> {
        let p: *mut u8 =
            unsafe { mi_heap_realloc(self.handle_ptr(), ptr.cast::<c_void>().as_ptr(), new_size) }
                .cast::<u8>();
        let p = NonNull::new(p)?;
        let sl = NonNull::slice_from_raw_parts(p, new_size);
        Some(sl)
    }

    pub fn realloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        let p: *mut u8 = unsafe {
            mi_heap_realloc_aligned(self.handle_ptr(), ptr.cast::<c_void>().as_ptr(), new_size, align)
        }
        .cast::<u8>();
        let p = NonNull::new(p)?;
        let sl = NonNull::slice_from_raw_parts(p, new_size);
        Some(sl)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn zalloc(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.calloc(1, size)
    }

    #[inline]
    pub fn zalloc_aligned(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.calloc_aligned(1, size, align)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn rezalloc(&self, ptr: NonNull<u8>, new_size: usize) -> Option<NonNull<[u8]>> {
        self.recalloc(ptr, 1, new_size)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn rezalloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        self.recalloc_aligned(ptr, 1, new_size, align)
    }
}
