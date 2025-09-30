//! System Memory allocation api traits for Blight-rs.
//!
//! Currently we have a realloc_array_* api that the implementer is expected to define,
//! Id like to benchmark this way of funnelilng all malloc/calloc calls through these functions as
//! opposed to having specific function calls for specific operations like how mimalloc does it.
//!
//! Though Rust compiler may be smart enough to inline and reduce all these function call indirections to simple
//! mimalloc calls
//!
use core::{ffi::c_void, ptr::NonNull};

use mimalloc_bindgen::api::mi_free;

/// A System Allocator that provides a c-style malloc/calloc realloc/recalloc free/free_array api.
///
/// Required methods to implement are [SystemAllocator::realloc_array], [SystemAllocator::realloc_array_aligned].
/// [SystemAllocator::realloc_array_zeroed] and [SystemAllocator::realloc_array_zeroed_aligned]
///
/// From: [https://manual.cs50.io/3/reallocarray](https://manual.cs50.io/3/reallocarray)
///
/// # realloc()
///
/// ***The realloc() function changes the size of the memory block pointed to by ptr to size bytes.
/// The contents of the memory will be unchanged in the range from the start of the region up to the minimum of the old and new sizes.***
///
/// - If the new size is larger than the old size, the added memory will not be initialized.
/// - If ptr is NULL, then the call is equivalent to malloc(size), for all values of size.
/// - If size is equal to zero, and ptr is not NULL, then the call is equivalent to free(ptr) (but see "Nonportable behavior" for portability issues).
/// - Unless ptr is NULL, it must have been returned by an earlier call to malloc or related functions. If the area pointed to was moved, a free(ptr) is done.
///
/// # reallocarray()
/// ***The reallocarray() function changes the size of (and possibly moves) the memory block pointed to by ptr to be large enough for an array of nmemb elements,
/// each of which is size bytes. It is equivalent to the call***
///
/// ```
///
/// realloc(ptr, nmemb * size);
///
/// ```
///
/// ***However, unlike that realloc() call, reallocarray() fails safely in the case where the multiplication would overflow.
/// If such an overflow occurs, reallocarray() returns an error.***
///
/// # RETURN VALUE
/// The malloc(), calloc(), realloc(), and reallocarray() functions return a pointer to the allocated memory,
/// which is suitably aligned for any type that fits into the requested size or less. On error, these functions return NULL and set errno.
/// Attempting to allocate more than PTRDIFF_MAX bytes is considered an error, as an object that large could cause later pointer subtraction to overflow.
///
///
/// ***Note that this realloc_array method does not return an error, and is expected to return None if
/// any error occurecd during allocation***
///
///
/// # Safety
///
/// The [SystemAllocator::free] and [SystemAllocator::free_array] methods are marked unsafe
/// as freeing any pointer outside the Drop api is an unsafe operation in Rust,
/// These methods have a default implementation that reduces down to a single call to mimalloc's [mi_free]
pub unsafe trait SystemAllocator {
    fn realloc_array(
        &self,
        ptr: Option<NonNull<u8>>,
        count: usize,
        size: usize,
    ) -> Option<NonNull<[u8]>>;

    fn realloc_array_zeroed(
        &self,
        ptr: Option<NonNull<u8>>,
        count: usize,
        size: usize,
    ) -> Option<NonNull<[u8]>>;

    fn realloc_array_aligned(
        &self,
        ptr: Option<NonNull<u8>>,
        count: usize,
        size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>>;

    fn realloc_array_zeroed_aligned(
        &self,
        ptr: Option<NonNull<u8>>,
        count: usize,
        size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>>;

    #[inline]
    fn malloc(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.realloc_array(None, 1, size)
    }

    #[inline]
    fn malloc_aligned(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.realloc_array_aligned(None, 1, size, align)
    }

    #[inline]
    fn calloc(&self, count: usize, size: usize) -> Option<NonNull<[u8]>> {
        self.realloc_array_zeroed(None, count, size)
    }

    #[inline]
    fn calloc_aligned(&self, count: usize, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.realloc_array_zeroed_aligned(None, count, size, align)
    }

    #[inline]
    fn recalloc(&self, ptr: NonNull<u8>, new_count: usize, size: usize) -> Option<NonNull<[u8]>> {
        self.realloc_array(Some(ptr), new_count, size)
    }

    #[inline]
    fn recalloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_count: usize,
        size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        self.realloc_array_aligned(Some(ptr), new_count, size, align)
    }

    #[inline]
    fn realloc(&self, ptr: NonNull<u8>, new_size: usize) -> Option<NonNull<[u8]>> {
        self.realloc_array(Some(ptr), 1, new_size)
    }

    #[inline]
    fn realloc_aligned(&self, ptr: NonNull<u8>, new_size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.realloc_array_aligned(Some(ptr), 1, new_size, align)
    }

    #[inline]
    fn zalloc(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.calloc(1, size)
    }

    #[inline]
    fn zalloc_aligned(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        self.calloc_aligned(1, size, align)
    }

    #[inline]
    fn rezalloc(&self, ptr: NonNull<u8>, new_size: usize) -> Option<NonNull<[u8]>> {
        self.recalloc(ptr, 1, new_size)
    }

    #[inline]
    fn rezalloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        self.recalloc_aligned(ptr, 1, new_size, align)
    }

    #[inline]
    unsafe fn free(&self, ptr: NonNull<u8>) {
        unsafe { mi_free(ptr.cast::<c_void>().as_ptr()) }
    }

    #[inline]
    unsafe fn free_array(&self, ptr: NonNull<[u8]>) {
        let p = ptr.cast::<u8>();
        unsafe { self.free(p) };
    }
}

///
/// # Safety
///
/// This is a allocation trait so you should know what you are doing before implementing this!
/// as such it is marked unsafe.
/// As an extra level of caution, these methods are expected to not check if the size given is
/// small, even the defintion for what 'small' means is defined by the implementation.
/// as a reference blight-rs uses mimalloc MI_SMALL_SIZE_MAX (which is about 1Kb - 1024 bytes - on x64 bit
/// architectures)
pub unsafe trait SmallAllocator {
    fn malloc_small(&self, size: usize) -> Option<NonNull<[u8]>> {
        self.try_malloc_small(size).ok()
    }
    fn try_malloc_small(&self, size: usize) -> anyhow::Result<NonNull<[u8]>>;
}
