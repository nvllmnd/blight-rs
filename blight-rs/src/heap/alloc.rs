use alloc::alloc::Allocator;
use anyhow::{Context, ensure};
use core::{
    borrow::Borrow,
    cell::Ref,
    ffi::c_void,
    marker::PhantomData,
    ops::Deref,
    ptr::{NonNull, null_mut},
};
use mimalloc_bindgen::api::{
    mi_arena_area, mi_heap_delete, mi_heap_destroy, mi_heap_get_backing, mi_heap_new, mi_heap_new_ex,
    mi_heap_t, mi_malloc, mi_reserve_os_memory, mi_reserve_os_memory_ex,
};

use crate::{
    api::VoidPtr,
    heap::{InnerHeap, handle::HeapHandle},
};

/// An abstraction over mimalloc's heap_id api, which allows for heaps to reclaim memory
/// between heaps with the same tag
#[repr(i32)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeapTag {
    #[default]
    Default,
    String,
    Object,
    Constant,
    BlockA,
    BlockB,
    BlockC,
    BlockD,
    Extended(i32),
}

impl HeapTag {
    const U32_DEFAULT: i32 = Self::Default.as_i32();
    const U32_STRING: i32 = Self::String.as_i32();

    const U32_OBJECT: i32 = Self::Object.as_i32();
    const U32_CONSTANT: i32 = Self::Constant.as_i32();

    const U32_BLOCKA: i32 = Self::BlockA.as_i32();
    const U32_BLOCKB: i32 = Self::BlockB.as_i32();

    const U32_BLOCKC: i32 = Self::BlockC.as_i32();
    const U32_BLOCKD: i32 = Self::BlockD.as_i32();

    pub const fn new(val: i32) -> Self {
        match val {
            Self::U32_DEFAULT => Self::Default,
            Self::U32_STRING => Self::String,

            Self::U32_OBJECT => Self::Object,
            Self::U32_CONSTANT => Self::Constant,
            Self::U32_BLOCKA => Self::BlockA,

            Self::U32_BLOCKB => Self::BlockB,
            Self::U32_BLOCKC => Self::BlockC,

            Self::U32_BLOCKD => Self::BlockD,
            _ => Self::Extended(val),
        }
    }

    pub const fn as_i32(&self) -> i32 {
        match self {
            HeapTag::Default => 0,
            HeapTag::String => 1,
            HeapTag::Object => 2,
            HeapTag::Constant => 3,
            HeapTag::BlockA => 4,
            HeapTag::BlockB => 5,
            HeapTag::BlockC => 6,
            HeapTag::BlockD => 7,
            HeapTag::Extended(n) => *n,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum HeapCleanup {
    #[default]
    Recycle,
    Destroy,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Heap {
    ty: HeapCleanup,
    inner: InnerHeap,
    tag: HeapTag,
}

impl Heap {
    #[inline]
    pub fn new() -> Self {
        Self::inner_new()
    }

    #[inline]
    pub fn recycle_with(tag: HeapTag) -> Self {
        Self::inner_new_ex(tag, HeapCleanup::Recycle)
    }

    #[inline]
    pub fn destroy_with(tag: HeapTag) -> Self {
        // NOTE: I have a feeilng the reason i was unable to destroy heaps previously is because
        // mimalloc proably does not allow anyone to destroy a heap that is tagged with the default
        // tag, as this would most likley also destroy the backing heap, which mimalloc does not
        // allow, and probably asserts the program and it crashes. So im going to put an assert
        // here just in case and see if that stil happens now...
        assert!(
            !matches!(tag, HeapTag::Default),
            "Cannot create a Destroyable Heap that is tagged with the default tag!"
        );
        Self::inner_new_ex(tag, HeapCleanup::Destroy)
    }

    pub const fn is_recycle(&self) -> bool {
        matches!(self.ty, HeapCleanup::Recycle)
    }

    pub const fn is_destroy(&self) -> bool {
        matches!(self.ty, HeapCleanup::Destroy)
    }

    /// Gets mimallocs backing heap for current thread.
    /// Its safe to assume 'static lifetime as mimalloc always has a backing heap until program exit
    pub fn get_backing() -> HeapHandle<'static> {
        let miheap = unsafe { mi_heap_get_backing() };
        let miheap =
            NonNull::new(miheap).expect("No backing heap to get! mi_heap_get_backing returned nullptr!");
        unsafe { HeapHandle::from_handle(miheap) }
    }

    #[inline]
    pub fn malloc(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.inner.malloc(size)
    }

    #[inline]
    pub fn malloc_aligned(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.inner.malloc_aligned(size, align)
    }

    #[inline]
    pub fn malloc_small(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.inner.malloc_small(size)
    }

    #[inline]
    pub fn try_malloc_small(&self, size: usize) -> anyhow::Result<NonNull<[u8]>> {
        self.inner.try_malloc_small(size)
    }

    #[inline]
    pub fn calloc(&self, count: usize, size: usize) -> Option<NonNull<[u8]>> {
        self.inner.calloc(count, size)
    }

    #[inline]
    pub fn calloc_aligned(&self, count: usize, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.inner.calloc_aligned(count, size, align)
    }

    #[inline]
    pub fn recalloc(&self, ptr: NonNull<u8>, new_count: usize, size: usize) -> Option<NonNull<[u8]>> {
        self.inner.recalloc(ptr, new_count, size)
    }

    #[inline]
    pub fn recalloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_count: usize,
        size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        self.inner.recalloc_aligned(ptr, new_count, size, align)
    }

    #[inline]
    pub fn realloc(&self, ptr: NonNull<u8>, new_size: usize) -> Option<NonNull<[u8]>> {
        self.inner.realloc(ptr, new_size)
    }

    #[inline]
    pub fn realloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        self.inner.realloc_aligned(ptr, new_size, align)
    }

    #[inline]
    pub fn zalloc(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.inner.calloc(1, size)
    }

    #[inline]
    pub fn zalloc_aligned(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.inner.zalloc_aligned(size, align)
    }

    #[inline]
    pub fn rezalloc(&self, ptr: NonNull<u8>, new_size: usize) -> Option<NonNull<[u8]>> {
        self.inner.recalloc(ptr, 1, new_size)
    }

    #[inline]
    pub fn rezalloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        // self.as_handle().recalloc_aligned(ptr, 1, new_size, align)
        self.inner.recalloc_aligned(ptr, 1, new_size, align)
    }

    pub const fn as_handle<'a, 'b: 'a>(&'b self) -> HeapHandle<'a> {
        unsafe { HeapHandle::from_handle(self.inner.0) }
    }

    fn inner_new() -> Self {
        let inner = unsafe { mi_heap_new() };
        let inner = NonNull::new(inner).expect("mi_heap_new returned null pointer!");
        let inner = InnerHeap(inner);

        Self {
            inner,
            ty: HeapCleanup::default(),
            tag: HeapTag::default(),
        }
    }

    fn inner_new_ex(tag: HeapTag, ty: HeapCleanup) -> Self {
        let allow_destroy = matches!(ty, HeapCleanup::Destroy);

        // TODO: Right now we always pass null for associated arena, need to flesh out and test
        // that api, will come back to this later
        let h = unsafe { mi_heap_new_ex(tag.as_i32(), allow_destroy, null_mut()) };
        let h = NonNull::new(h)
            .expect("mi_heap_new_ex returned null! Check that arguments passed to it are valid!");
        Self {
            tag,
            ty,
            inner: InnerHeap(h),
        }
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        match self.ty {
            HeapCleanup::Destroy => unsafe { mi_heap_destroy(self.inner.handle_ptr()) },
            HeapCleanup::Recycle => unsafe { mi_heap_delete(self.inner.handle_ptr()) },
        }
    }
}

/// A private wrapper type that handles mimalloc mi_heap_* api calls
unsafe impl Allocator for Heap {
    fn allocate(&self, layout: core::alloc::Layout) -> Result<NonNull<[u8]>, alloc::alloc::AllocError> {
        let size = layout.size();
        let align = layout.align();
        self.inner
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
        self.inner
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
        self.inner
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
        self.inner
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
        self.inner
            .recalloc_aligned(ptr, 1, size, align)
            .ok_or(alloc::alloc::AllocError)
    }
}

#[cfg(test)]
mod tests {
    use core::ptr::NonNull;

    use crate::heap::alloc::{Heap, HeapTag};

    #[repr(C)]
    #[derive(Debug, Default, Clone, Copy)]
    struct Point3D {
        x: i32,
        y: i32,
        z: i32,
        w: i32,
    }

    #[test]
    fn heap_alloc_then_delete() {
        const LEN: usize = 5;
        let h = Heap::recycle_with(HeapTag::Object);
        if let Some(pm) = h.calloc_aligned(
            LEN,
            core::mem::size_of::<Point3D>(),
            core::mem::align_of::<Point3D>(),
        ) {
            let pm = pm.cast::<Point3D>();
            let mut pms = NonNull::slice_from_raw_parts(pm, LEN);
            let xs = unsafe { pms.as_mut() };
            xs.copy_from_slice(
                &[Point3D {
                    x: 1,
                    y: 2,
                    z: 3,
                    w: 4,
                }; 5],
            );
        } else {
            panic!("calloc_aligned returned None!");
        }
    }
}
