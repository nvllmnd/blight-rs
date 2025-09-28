use core::{ffi::c_void, ptr::NonNull};

use alloc::{alloc::Allocator, boxed::Box, rc::Rc};
use anyhow::{Context, ensure};
use mimalloc_bindgen::api::{
    MI_SMALL_WSIZE_MAX, mi_calloc, mi_calloc_aligned, mi_heap_calloc, mi_heap_calloc_aligned,
    mi_heap_delete, mi_heap_malloc, mi_heap_malloc_aligned, mi_heap_malloc_small, mi_heap_new,
    mi_heap_t, mi_malloc, mi_malloc_small, mi_reallocarray, mi_recalloc, mi_rezalloc, mi_zalloc,
    mi_zalloc_small,
};

// NOTE: bindgen didnt seem to be able to generate MI_SMALL_SIZE_MAX, so this is custom impl
pub const MIMALLOC_SMALL_SIZE_MAX: usize = MI_SMALL_WSIZE_MAX as usize * size_of::<usize>();

#[repr(transparent)]
pub struct Heap(NonNull<mi_heap_t>);

impl Heap {
    pub fn new() -> Self {
        let h = unsafe { mi_heap_new() };
        let h = NonNull::new(h).expect("mi_heap_new returned nullptr!");
        Self(h)
    }

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
        let p =
            NonNull::new(p).context("mi_heap_malloc_small returned nullptr or was too large!")?;
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
        let p =
            unsafe { mi_heap_calloc_aligned(self.handle_ptr(), count, size, align) }.cast::<u8>();
        let p = NonNull::new(p)?;
        let sl = NonNull::slice_from_raw_parts(p, size);
        Some(sl)
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        unsafe { mi_heap_delete(self.0.as_ptr()) };
    }
}

unsafe impl Allocator for Heap {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, alloc::alloc::AllocError> {
        let size = layout.size();
        let align = layout.align();
        self.malloc_aligned(size, align)
            .ok_or(alloc::alloc::AllocError)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: core::alloc::Layout) {}
}

pub fn malloc(size_bytes: usize) -> Option<NonNull<u8>> {
    let p = unsafe { mi_malloc(size_bytes) } as *mut u8;
    NonNull::new(p)
}

/// Allocates with mimalloc_small if size_bytes is less than or equal to 1024 bytes on 64 bit systems ([MIMALLOC_SMALL_SIZE_MAX])
/// Returns Non if out of memory of size_bytes was too big
pub fn malloc_small(size_bytes: usize) -> Option<NonNull<u8>> {
    try_malloc_small(size_bytes).ok()
}

pub fn try_malloc_small(size_bytes: usize) -> anyhow::Result<NonNull<u8>> {
    ensure!(
        size_bytes <= MIMALLOC_SMALL_SIZE_MAX,
        "size_bytes must be smaller than MIMALLOC_SMALL_SIZE_MAX (1024 bytes)"
    );

    let p = unsafe { mi_malloc_small(size_bytes) as *mut u8 };
    let p = NonNull::new(p).context("mi_malloc_small returned nullptr!")?;
    Ok(p)
}

pub fn calloc(count: usize, size_bytes: usize) -> Option<NonNull<u8>> {
    let p = unsafe { mi_calloc(count, size_bytes) } as *mut u8;
    NonNull::new(p)
}

/// Allocates with mimalloc_small if size_bytes is less than or equal to 1024 bytes on 64 bit systems ([MIMALLOC_SMALL_SIZE_MAX])
/// panics if size_bytes >= [MIMALLOC_SMALL_SIZE_MAX]
pub fn calloc_aligned(count: usize, size_bytes: usize, align: usize) -> Option<NonNull<u8>> {
    let p = unsafe { mi_calloc_aligned(count, size_bytes, align).cast::<u8>() };
    NonNull::new(p)
}

pub fn recalloc(ptr: NonNull<u8>, new_count: usize, size_bytes: usize) -> Option<NonNull<[u8]>> {
    let ptr = ptr.cast::<c_void>().as_ptr();
    let ptr = unsafe { mi_recalloc(ptr, new_count, size_bytes) }.cast::<u8>();
    let ptr = NonNull::new(ptr)?;
    let sl = NonNull::slice_from_raw_parts(ptr, new_count);
    Some(sl)
}
