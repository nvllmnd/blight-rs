use core::{ffi::c_void, ptr::NonNull};

use alloc::{alloc::Allocator, rc::Rc};
use anyhow::{Context, ensure};
use mimalloc_bindgen::api::{
    MI_SMALL_WSIZE_MAX, mi_calloc, mi_calloc_aligned, mi_malloc_small, mi_recalloc,
};

// NOTE: bindgen didnt seem to be able to generate MI_SMALL_SIZE_MAX, so this is custom impl
pub const MIMALLOC_SMALL_SIZE_MAX: usize = MI_SMALL_WSIZE_MAX as usize * size_of::<usize>();

/// Allocates with mimalloc_small if size_bytes is less than or equal to 1024 bytes on 64 bit systems ([MIMALLOC_SMALL_SIZE_MAX])
/// Returns Non if out of memory of size_bytes was too big
pub fn malloc_small(size_bytes: usize) -> Option<NonNull<[u8]>> {
    try_malloc_small(size_bytes).ok()
}

pub fn try_malloc_small(size_bytes: usize) -> anyhow::Result<NonNull<[u8]>> {
    ensure!(
        size_bytes <= MIMALLOC_SMALL_SIZE_MAX,
        "size_bytes must be smaller than MIMALLOC_SMALL_SIZE_MAX (1024 bytes)"
    );

    let p = unsafe { mi_malloc_small(size_bytes) as *mut u8 };
    let p = NonNull::new(p).context("mi_malloc_small returned nullptr!")?;
    let sl = NonNull::slice_from_raw_parts(p, size_bytes);
    Ok(sl)
}

pub fn calloc(count: usize, size_bytes: usize) -> Option<NonNull<[u8]>> {
    let p = unsafe { mi_calloc(count, size_bytes) } as *mut u8;
    let p = NonNull::new(p)?;
    let sl = NonNull::slice_from_raw_parts(p, count * size_bytes);
    Some(sl)
}

/// Allocates with mimalloc_small if size_bytes is less than or equal to 1024 bytes on 64 bit systems ([MIMALLOC_SMALL_SIZE_MAX])
/// panics if size_bytes >= [MIMALLOC_SMALL_SIZE_MAX]
pub fn calloc_aligned(count: usize, size_bytes: usize, align: usize) -> Option<NonNull<[u8]>> {
    let p = unsafe { mi_calloc_aligned(count, size_bytes, align).cast::<u8>() };
    let p = NonNull::new(p)?;
    let sl = NonNull::slice_from_raw_parts(p, count * size_bytes);
    Some(sl)
}

pub fn recalloc(ptr: NonNull<u8>, new_count: usize, size_bytes: usize) -> Option<NonNull<[u8]>> {
    let ptr = ptr.cast::<c_void>().as_ptr();
    let ptr = unsafe { mi_recalloc(ptr, new_count, size_bytes) }.cast::<u8>();
    let ptr = NonNull::new(ptr)?;
    let sl = NonNull::slice_from_raw_parts(ptr, new_count);
    Some(sl)
}
