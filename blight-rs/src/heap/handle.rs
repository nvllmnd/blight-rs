use core::{marker::PhantomData, ptr::NonNull};

use alloc::alloc::Allocator;
use mimalloc_bindgen::api::mi_heap_t;

use crate::heap::{InnerHeap, alloc::Heap};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeapHandle<'heap> {
    ptr: InnerHeap,
    _pd: PhantomData<&'heap Heap>,
}

impl HeapHandle<'_> {
    pub(crate) const unsafe fn from_handle(handle: NonNull<mi_heap_t>) -> Self {
        Self {
            ptr: InnerHeap(handle),
            _pd: PhantomData,
        }
    }

    const fn inner(&self) -> &InnerHeap {
        &self.ptr
    }

    #[inline]
    pub fn malloc(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.inner().malloc(size)
    }

    #[inline]
    pub fn malloc_aligned(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.inner().malloc_aligned(size, align)
    }

    #[inline]
    pub fn malloc_small(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.inner().malloc_small(size)
    }

    #[inline]
    pub fn try_malloc_small(&self, size: usize) -> anyhow::Result<NonNull<[u8]>> {
        self.inner().try_malloc_small(size)
    }

    #[inline]
    pub fn calloc(&self, count: usize, size: usize) -> Option<NonNull<[u8]>> {
        self.inner().calloc(count, size)
    }

    #[inline]
    pub fn calloc_aligned(&self, count: usize, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.inner().calloc_aligned(count, size, align)
    }

    #[inline]
    pub fn recalloc(&self, ptr: NonNull<u8>, new_count: usize, size: usize) -> Option<NonNull<[u8]>> {
        self.inner().recalloc(ptr, new_count, size)
    }

    #[inline]
    pub fn recalloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_count: usize,
        size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        self.inner().recalloc_aligned(ptr, new_count, size, align)
    }

    #[inline]
    pub fn realloc(&self, ptr: NonNull<u8>, new_size: usize) -> Option<NonNull<[u8]>> {
        self.inner().realloc(ptr, new_size)
    }

    #[inline]
    pub fn realloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        self.inner().realloc_aligned(ptr, new_size, align)
    }

    #[inline]
    pub fn zalloc(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.inner().calloc(1, size)
    }

    #[inline]
    pub fn zalloc_aligned(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.inner().zalloc_aligned(size, align)
    }

    #[inline]
    pub fn rezalloc(&self, ptr: NonNull<u8>, new_size: usize) -> Option<NonNull<[u8]>> {
        self.inner().recalloc(ptr, 1, new_size)
    }

    #[inline]
    pub fn rezalloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        self.inner().recalloc_aligned(ptr, 1, new_size, align)
    }
}

// TODO: This is pretty much a copy+paste of above impl Allocator for Heap,
// there is probably a better way to do this which will be obvious to me some time from now lol
unsafe impl Allocator for HeapHandle<'_> {
    fn allocate(&self, layout: core::alloc::Layout) -> Result<NonNull<[u8]>, alloc::alloc::AllocError> {
        let size = layout.size();
        let align = layout.align();
        self.inner()
            .malloc_aligned(size, align)
            .ok_or(alloc::alloc::AllocError)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: core::alloc::Layout) {}

    fn allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, alloc::alloc::AllocError> {
        let size = layout.size();
        let align = layout.align();
        self.inner()
            .calloc_aligned(1, size, align)
            .ok_or(alloc::alloc::AllocError)
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, alloc::alloc::AllocError> {
        core::debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
        );
        let size = new_layout.size();
        let align = new_layout.align();
        self.inner()
            .realloc_aligned(ptr, size, align)
            .ok_or(alloc::alloc::AllocError)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, alloc::alloc::AllocError> {
        core::debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
        );
        let size = new_layout.size();
        let align = new_layout.align();
        self.inner()
            .recalloc_aligned(ptr, 1, size, align)
            .ok_or(alloc::alloc::AllocError)
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, alloc::alloc::AllocError> {
        core::debug_assert!(
            new_layout.size() <= old_layout.size(),
            "`new_layout.size()` must be smaller than or equal to `old_layout.size()`"
        );
        let size = new_layout.size();
        let align = new_layout.align();
        self.inner()
            .recalloc_aligned(ptr, 1, size, align)
            .ok_or(alloc::alloc::AllocError)
    }
}
