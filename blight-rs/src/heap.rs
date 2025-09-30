use alloc::alloc::Allocator;
use anyhow::{Context, ensure};
use core::{
    ffi::c_void,
    marker::PhantomData,
    ptr::{NonNull, null_mut},
};
use mimalloc_bindgen::api::{
    mi_arena_area, mi_heap_calloc, mi_heap_calloc_aligned, mi_heap_delete, mi_heap_destroy,
    mi_heap_get_backing, mi_heap_malloc, mi_heap_malloc_aligned, mi_heap_malloc_small, mi_heap_new,
    mi_heap_new_ex, mi_heap_realloc, mi_heap_realloc_aligned, mi_heap_recalloc,
    mi_heap_recalloc_aligned, mi_heap_t, mi_malloc, mi_reserve_os_memory, mi_reserve_os_memory_ex,
};

use crate::mimalloc::MIMALLOC_SMALL_SIZE_MAX;

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
    Delete,
    Destroy,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Heap {
    ty: HeapCleanup,
    inner: InnerHeap,
    tag: HeapTag,
}

impl Heap {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn is_delete(&self) -> bool {
        matches!(self.ty, HeapCleanup::Delete)
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
        HeapHandle {
            ptr: InnerHeap(miheap),
            _pd: PhantomData,
        }
    }
}

impl Default for Heap {
    fn default() -> Self {
        const DEFAULT_TAG: i32 = HeapTag::Default.as_i32();
        // TODO: Right now we always pass null for associated arena, need to flesh out and test
        // that api, will come back to this later
        let h = unsafe { mi_heap_new_ex(DEFAULT_TAG, false, null_mut()) };
        let h = NonNull::new(h).expect("mi_heap_new_ex(0, false, null_mut()) returned null! Check that arguments passed to it are valid!");
        Self {
            ty: HeapCleanup::Delete,
            inner: InnerHeap(h),
            tag: HeapTag::Default,
        }
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        match self.ty {
            HeapCleanup::Destroy => unsafe { mi_heap_destroy(self.inner.handle_ptr()) },
            HeapCleanup::Delete => unsafe { mi_heap_delete(self.inner.handle_ptr()) },
        }
    }
}

type MiHeapPtr = NonNull<mi_heap_t>;

/// A private wrapper type that handles mimalloc mi_heap_* api calls
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InnerHeap(MiHeapPtr);

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

pub fn malloc(size_bytes: usize) -> Option<NonNull<u8>> {
    let p = unsafe { mi_malloc(size_bytes) } as *mut u8;
    NonNull::new(p)
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeapHandle<'heap> {
    ptr: InnerHeap,
    _pd: PhantomData<&'heap InnerHeap>,
}

impl HeapHandle<'_> {
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

pub struct OsArenaOpts {
    /// Size in bytes of arena
    pub size: usize,
    /// Commit memory upfront?
    pub commit: bool,
    /// Is this arena exclusive?
    pub exclusive: bool,
    /// Allow Large OS pages (2MB) to be used?
    pub allow_large: bool,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OsArena(NonNull<c_void>);

impl OsArena {
    /// Calls mimalloc's mi_reserve_os_memory to have it allocate an initial block of memory
    /// that it will use before calling a system allocator again to request more memory
    /// This wont return an instance of OsArena, as this mimalloc function does not return a
    /// mi_arena_id to associate an arena with.
    ///
    /// returns true on success, or false if out of memory (ENOMEM was returned from inner mimalloc
    /// api call)
    pub fn reserve(size_bytes: usize, commit_upfront: bool, allow_large_pages: bool) -> bool {
        let err: i32 = unsafe { mi_reserve_os_memory(size_bytes, commit_upfront, allow_large_pages) };
        if err == 0 { true } else { false }
    }

    pub fn new(opts: OsArenaOpts) -> anyhow::Result<Self> {
        let mut arena_id: *mut c_void = null_mut();
        let OsArenaOpts {
            size,
            commit,
            exclusive,
            allow_large,
        } = opts;
        let err: i32 =
            unsafe { mi_reserve_os_memory_ex(size, commit, allow_large, exclusive, &raw mut arena_id) };
        ensure!(err == 0, "mi_reserve_os_memory_ex returned non-zero error code!");
        let arena =
            NonNull::new(arena_id).context("Arena Id returned from mi_reserve_os_memory_ex is null!")?;
        Ok(Self(arena))
    }

    /// Calls mi_arena_area to get the begin address and size of this arena
    /// and returns them as a NonNull slice of u8
    pub fn slice_ptr(&self) -> NonNull<[u8]> {
        let mut size = 0;
        let start = unsafe { mi_arena_area(self.0.as_ptr(), &raw mut size) };
        let start = NonNull::new(start)
            .expect("mi_arena_area returned a null base address!")
            .cast::<u8>();

        debug_assert!(size != 0);

        NonNull::slice_from_raw_parts(start, size)
    }

    #[inline]
    pub fn size_bytes(&self) -> usize {
        self.slice_ptr().len()
    }
}
