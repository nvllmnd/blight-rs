//! System Memory allocation api traits for Blight-rs.
//!
//! Currently we have a (z/a)llocate_blocks* api that the implementer is expected to define,
//! Id like to benchmark this way of funnelilng all malloc/calloc calls through these functions as
//! opposed to having specific function calls for specific operations like how mimalloc does it.
//!
//! Though Rust compiler may be smart enough to inline and reduce all these function call indirections to simple
//! mimalloc calls
//!
use core::{ffi::c_void, ptr::NonNull};

use mimalloc_bindgen::api::mi_free;

// TODO: Come back to this, for now we are not implementing this...

pub type Ptr<T> = NonNull<T>;
pub type SlicePtr<T> = Ptr<[T]>;
pub type AnyPtr = Ptr<u8>;
pub type BlockPtr = SlicePtr<u8>;

pub type AllocResult = Option<MemoryChunk>;

pub type VoidPtr = Ptr<c_void>;

/// A chunk of memory returned by blight system allocator
/// Can contain blocks of memory that make up the entire chunk.
/// each chunk can be indexed or retrieved via [MemoryChunk::block_at]
///
/// This type does not implement drop and is up to the user to cleanup
/// when they are finished with it, or wrap it in thier own RAII type
///
/// NOTE: This struct is not concered with types, even though it has generic constructor
/// functions, these constructor functions are generic only to serve as a conveience for attaining
/// the proper size and alignments
#[derive(Debug, Clone, Copy)]
pub enum MemoryChunk {
    Scalar {
        /// Pointer slice to start of memory block, len of
        /// this slice is the entire size of the block
        ptr: BlockPtr,

        /// Alignment of entire memory chunk.
        /// as usual, must be a power of 2. Set to 1 if alignment is not specified
        alignment: usize,
    },
    Blocks {
        /// Pointer slice to start of memory block, len of
        /// this slice is the entire size of the block (including its elements)
        ptr: BlockPtr,

        /// size in bytes of each block.
        /// this field is 0 in the case of  this memory chunk being a scalar chunk and
        /// being composed of 0 blocks
        block_size: usize,

        /// Count of blocks this memory chunk is composed of
        /// this field will be 0 if this memory chunk contains only a scalar block
        /// (thus block_size == self.ptr.len())
        count: usize,

        /// Alignment of entire memory chunk. used to give proper pointer address of item member
        /// as usual, must be a power of 2. Set to 1 if alignment is not specified
        alignment: usize,
    },
}

impl MemoryChunk {
    pub const fn bytes(ptr: BlockPtr) -> Self {
        Self::Scalar { ptr, alignment: 1 }
    }

    pub const fn bytes_aligned(ptr: BlockPtr, alignment: usize) -> Self {
        Self::Scalar { ptr, alignment }
    }

    pub const fn blocks_bytes(ptr: BlockPtr, block_size: usize, count: usize) -> Self {
        Self::Blocks {
            ptr,
            block_size,
            count,
            alignment: 1,
        }
    }

    pub const fn blocks_bytes_aligned(
        ptr: BlockPtr,
        block_size: usize,
        count: usize,
        alignment: usize,
    ) -> Self {
        Self::Blocks {
            ptr,
            block_size,
            count,
            alignment,
        }
    }

    pub const fn blocks<T>(ptr: BlockPtr, count: usize) -> Self
    where
        T: Sized,
    {
        Self::Blocks {
            ptr,
            block_size: core::mem::size_of::<T>(),
            count,
            alignment: core::mem::align_of::<T>(),
        }
    }

    pub const fn scalar<T>(ptr: BlockPtr) -> Self
    where
        Self: Sized,
    {
        assert!(
            ptr.len() == core::mem::size_of::<T>(),
            "pointer given to MemoryChunk::new must be the same byte length as of size T!"
        );
        Self::Scalar {
            ptr,
            alignment: core::mem::align_of::<T>(),
        }
    }

    pub const fn block_size(&self) -> usize {
        match self {
            MemoryChunk::Scalar { .. } => 0,
            MemoryChunk::Blocks { block_size, .. } => *block_size,
        }
    }

    pub const fn block_ptr(&self) -> BlockPtr {
        match self {
            Self::Scalar { ptr, .. } => *ptr,
            Self::Blocks { ptr, .. } => *ptr,
        }
    }

    pub const fn ptr(&self) -> AnyPtr {
        self.block_ptr().cast::<u8>()
    }

    pub const fn count(&self) -> usize {
        match self {
            Self::Scalar { .. } => 0,
            Self::Blocks { count, .. } => *count,
        }
    }

    pub const fn alignment(&self) -> usize {
        match self {
            MemoryChunk::Scalar { alignment, .. } => *alignment,
            MemoryChunk::Blocks { alignment, .. } => *alignment,
        }
    }

    pub const fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar { .. })
    }

    pub const fn is_blocks(&self) -> bool {
        !self.is_scalar()
    }

    /// Returns a non null slice pointer to block at given index param.
    /// returns pointer to start of memory block if this is a [MemoryChunk::Scalar],
    /// otherwise the slice pointer to block at offset of
    /// index * [MemoryChunk::Blocks::block_size],
    /// or else None if out of range (index >= [MemoryChunk::Blocks::count]),
    pub fn block_at(&self, index: usize) -> Option<BlockPtr> {
        match self {
            MemoryChunk::Scalar { ptr, .. } => Some(*ptr),
            MemoryChunk::Blocks {
                ptr,
                block_size,
                count,
                alignment,
            } if index < *count => {
                debug_assert!(alignment.is_power_of_two(), "Alignment must be a power of 2!");

                let ptr = ptr.cast::<u8>();
                let ptr = unsafe { ptr.add(index * block_size) };

                let alignment = *alignment;

                let offset = ptr.align_offset(alignment);
                let block_size = *block_size;

                if offset < block_size {
                    let ptr = unsafe { ptr.add(offset) };
                    let sl = BlockPtr::slice_from_raw_parts(ptr, block_size);

                    Some(sl)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

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
/// // realloc(ptr, nmemb * size);
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
    /// // realloc(ptr, nmemb * size);
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
    fn allocate_chunk(
        &self,
        resize_ptr: Option<NonNull<u8>>,
        block_count: usize,
        size: usize,
    ) -> AllocResult;

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
    /// // realloc(ptr, nmemb * size);
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
    fn zallocate_chunk(
        &self,
        resize_ptr: Option<NonNull<u8>>,
        block_count: usize,
        size: usize,
    ) -> AllocResult;

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
    /// // realloc(ptr, nmemb * size);
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
    fn allocate_chunk_aligned(
        &self,
        resize_ptr: Option<NonNull<u8>>,
        block_count: usize,
        size: usize,
        align: usize,
    ) -> AllocResult;

    ///
    /// like [SystemAllocator::allocate_blocks_aligned], but zeros all memory
    /// This is a separate method to avoid redundant zeroing by implementing this method naively
    /// by calling [SystemAllocator::allocate_blocks_aligned] and zeroing memory before returning
    /// pointer to that block
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
    /// // realloc(ptr, nmemb * size);
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
    /// ***Note that [SystemAllocator::zallocate_blocks_aligned] method does not return an error, and is expected to return None if
    /// any error occurecd during allocation***
    ///
    fn zallocate_chunk_aligned(
        &self,
        ptr: Option<NonNull<u8>>,
        count: usize,
        size: usize,
        align: usize,
    ) -> AllocResult;

    #[inline]
    fn malloc(&self, size: usize) -> AllocResult {
        self.allocate_chunk(None, 1, size)
    }

    #[inline]
    fn malloc_aligned(&self, size: usize, align: usize) -> AllocResult {
        self.allocate_chunk_aligned(None, 1, size, align)
    }

    #[inline]
    fn calloc(&self, count: usize, size: usize) -> AllocResult {
        self.zallocate_chunk(None, count, size)
    }

    #[inline]
    fn calloc_aligned(&self, count: usize, size: usize, align: usize) -> AllocResult {
        self.zallocate_chunk_aligned(None, count, size, align)
    }

    #[inline]
    fn recalloc(&self, ptr: NonNull<u8>, new_count: usize, size: usize) -> AllocResult {
        self.zallocate_chunk(Some(ptr), new_count, size)
    }

    #[inline]
    fn recalloc_aligned(
        &self,
        ptr: NonNull<u8>,
        new_count: usize,
        size: usize,
        align: usize,
    ) -> AllocResult {
        self.zallocate_chunk_aligned(Some(ptr), new_count, size, align)
    }

    #[inline]
    fn realloc(&self, ptr: NonNull<u8>, new_size: usize) -> AllocResult {
        self.allocate_chunk(Some(ptr), 1, new_size)
    }

    #[inline]
    fn realloc_aligned(&self, ptr: NonNull<u8>, new_size: usize, align: usize) -> AllocResult {
        self.allocate_chunk_aligned(Some(ptr), 1, new_size, align)
    }

    #[inline]
    fn zalloc(&self, size: usize) -> AllocResult {
        self.calloc(1, size)
    }

    #[inline]
    fn zalloc_aligned(&self, size: usize, align: usize) -> AllocResult {
        self.calloc_aligned(1, size, align)
    }

    #[inline]
    fn rezalloc(&self, ptr: NonNull<u8>, new_size: usize) -> AllocResult {
        self.recalloc(ptr, 1, new_size)
    }

    #[inline]
    fn rezalloc_aligned(&self, ptr: NonNull<u8>, new_size: usize, align: usize) -> AllocResult {
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

    #[inline]
    unsafe fn free_chunk(&self, chunk: &MemoryChunk) {
        unsafe { self.free_array(chunk.block_ptr()) }
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
    fn malloc_small(&self, size: usize) -> AllocResult {
        self.try_malloc_small(size).ok()
    }

    fn try_malloc_small(&self, size: usize) -> anyhow::Result<MemoryChunk>;
}
