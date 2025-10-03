//!  
//!  This module wraps most mimalloc functions with safe rust functions
//!
//!
use core::{
    ffi::c_void,
    mem::transmute_copy,
    ops::Deref,
    ptr::{self, NonNull, null_mut},
};

use alloc::{alloc::Allocator, rc::Rc};
use anyhow::{Context, ensure};
use mimalloc_bindgen::api::{
    MI_SMALL_WSIZE_MAX, mi_calloc, mi_calloc_aligned, mi_free, mi_good_size, mi_malloc,
    mi_malloc_aligned, mi_malloc_small, mi_option_disable, mi_option_e,
    mi_option_e_mi_option_abandoned_page_purge, mi_option_e_mi_option_allow_large_os_pages,
    mi_option_e_mi_option_arena_eager_commit, mi_option_e_mi_option_arena_purge_mult,
    mi_option_e_mi_option_arena_reserve, mi_option_e_mi_option_destroy_on_exit,
    mi_option_e_mi_option_disallow_arena_alloc, mi_option_e_mi_option_disallow_os_alloc,
    mi_option_e_mi_option_eager_commit, mi_option_e_mi_option_eager_commit_delay,
    mi_option_e_mi_option_limit_os_alloc, mi_option_e_mi_option_max_errors,
    mi_option_e_mi_option_max_warnings, mi_option_e_mi_option_os_tag,
    mi_option_e_mi_option_purge_decommits, mi_option_e_mi_option_purge_delay,
    mi_option_e_mi_option_reserve_huge_os_pages, mi_option_e_mi_option_reserve_huge_os_pages_at,
    mi_option_e_mi_option_reserve_os_memory, mi_option_e_mi_option_retry_on_oom,
    mi_option_e_mi_option_show_errors, mi_option_e_mi_option_show_stats,
    mi_option_e_mi_option_use_numa_nodes, mi_option_e_mi_option_verbose,
    mi_option_e_mi_option_visit_abandoned, mi_option_enable, mi_option_get, mi_option_get_clamp,
    mi_option_get_size, mi_option_is_enabled, mi_option_set, mi_option_set_default,
    mi_option_set_enabled, mi_option_set_enabled_default, mi_realloc, mi_realloc_aligned, mi_recalloc,
    mi_recalloc_aligned, mi_stats_print, mi_usable_size,
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
pub fn malloc(size: usize) -> Option<NonNull<[u8]>> {
    let p = unsafe { mi_malloc(size) } as *mut u8;
    let p = NonNull::new(p)?;
    let sl = NonNull::slice_from_raw_parts(p, size);
    Some(sl)
}
pub fn malloc_aligned(size: usize, align: usize) -> Option<NonNull<[u8]>> {
    let p = unsafe { mi_malloc_aligned(size, align) } as *mut u8;
    let p = NonNull::new(p)?;
    let sl = NonNull::slice_from_raw_parts(p, size);
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
    let sl = NonNull::slice_from_raw_parts(ptr, new_count * size_bytes);
    Some(sl)
}

pub fn print_stats() {
    unsafe { mi_stats_print(ptr::null_mut()) }
}

pub fn recalloc_aligned(
    ptr: NonNull<u8>,
    new_count: usize,
    size_bytes: usize,
    align: usize,
) -> Option<NonNull<[u8]>> {
    debug_assert!(align.is_power_of_two(), "alignment must be a power of 2!");

    let ptr = ptr.cast::<c_void>().as_ptr();
    let ptr = unsafe { mi_recalloc_aligned(ptr, new_count, size_bytes, align) }.cast::<u8>();
    let ptr = NonNull::new(ptr)?;
    let sl = NonNull::slice_from_raw_parts(ptr, new_count * size_bytes);
    Some(sl)
}

pub fn realloc_aligned(ptr: NonNull<u8>, new_size: usize, align: usize) -> Option<NonNull<[u8]>> {
    debug_assert!(align.is_power_of_two(), "alignment must be a power of 2!");

    let ptr = ptr.cast::<c_void>().as_ptr();
    let ptr = unsafe { mi_realloc_aligned(ptr, new_size, align) }.cast::<u8>();
    let ptr = NonNull::new(ptr)?;
    let sl = NonNull::slice_from_raw_parts(ptr, new_size);
    Some(sl)
}

pub fn realloc(ptr: NonNull<u8>, size_bytes: usize) -> Option<NonNull<[u8]>> {
    let ptr = ptr.cast::<c_void>().as_ptr();
    let ptr = unsafe { mi_realloc(ptr, size_bytes) }.cast::<u8>();
    let ptr = NonNull::new(ptr)?;
    let sl = NonNull::slice_from_raw_parts(ptr, size_bytes);
    Some(sl)
}

///
/// # Safety
///
/// Deallocates memory pointed to by ptr param by calling mi_free(ptr)
/// All memory deallocation outside of Rust's Drop impls are unsafe
/// as they fall outside of what the borrow checker can ensure
pub unsafe fn mifree(ptr: NonNull<u8>) {
    unsafe {
        mi_free(ptr.cast::<c_void>().as_ptr());
    }
}

/// Return the available bytes in a memory block.
///
/// Parameters
/// ptr :: Pointer to previously allocated memory
///
/// Returns the available bytes in the memory block, or 0 if p was NULL.
/// The returned size can be used to call mi_expand successfully. The returned size is always at least equal to the allocated size of p.
///
/// See also
/// - _msize (Windows)
/// - malloc_usable_size (Linux)
/// - mi_good_size()
/// - [good_size]
#[inline]
pub fn usable_size(ptr: NonNull<u8>) -> usize {
    unsafe { mi_usable_size(ptr.cast::<c_void>().as_ptr()) }
}

/// Return the used allocation size.
///
/// Parameters
/// size	The minimal required size in bytes.
/// Returns
/// the size n that will be allocated, where n >= size.
/// Generally, mi_usable_size(mi_malloc(size)) == mi_good_size(size). This can be used to reduce internal wasted space when allocating buffers for example.
///
/// See also
/// - mi_usable_size()
/// - [usable_size]
#[inline]
pub fn good_size(size: usize) -> usize {
    unsafe { mi_good_size(size) }
}

#[repr(u32)]
pub enum MiOptionType {
    /// Print error messages.
    ShowErrors = mi_option_e_mi_option_show_errors,
    /// Print statistics on termination.
    ShowStats = mi_option_e_mi_option_show_stats,
    /// Print verbose messages.
    Verbose = mi_option_e_mi_option_verbose,
    /// issue at most N error messages
    MaxErrors = mi_option_e_mi_option_max_errors,
    /// issue at most N warning messages
    MaxWarnings = mi_option_e_mi_option_max_warnings,
    /// reserve N huge OS pages (1GiB pages) at startup
    ReserveHugeOsPages = mi_option_e_mi_option_reserve_huge_os_pages,
    /// Reserve N huge OS pages at a specific NUMA node N.
    ReserveHugeOsPagesAt = mi_option_e_mi_option_reserve_huge_os_pages_at,
    /// reserve specified amount of OS memory in an arena at startup (internally, this value is in KiB; use mi_option_get_size)
    ReserveOsMemory = mi_option_e_mi_option_reserve_os_memory,
    /// allow large (2 or 4 MiB) OS pages, implies eager commit. If false, also disables THP for the process.
    AllowLargeOsPages = mi_option_e_mi_option_allow_large_os_pages,
    /// should a memory purge decommit? (=1). Set to 0 to use memory reset on a purge (instead of decommit)
    PurgeDecommits = mi_option_e_mi_option_purge_decommits,
    /// initial memory size for arena reservation (= 1 GiB on 64-bit) (internally, this value is in KiB; use mi_option_get_size)
    ArenaReserve = mi_option_e_mi_option_arena_reserve,
    /// tag used for OS logging (macOS only for now) (=100)
    OsTag = mi_option_e_mi_option_os_tag,
    /// retry on out-of-memory for N milli seconds (=400), set to 0 to disable retries. (only on windows)
    RetryOnOOM = mi_option_e_mi_option_retry_on_oom,

    /// eager commit segments? (after eager_commit_delay segments) (enabled by default).
    EagerCommit = mi_option_e_mi_option_eager_commit,
    /// the first N segments per thread are not eagerly committed (but per page in the segment on demand)
    EagerCommitDelay = mi_option_e_mi_option_eager_commit_delay,
    /// eager commit arenas? Use 2 to enable just on overcommit systems (=2)
    ArenaEagerCommit = mi_option_e_mi_option_arena_eager_commit,
    /// immediately purge delayed purges on thread termination
    AbandonedPagePurge = mi_option_e_mi_option_abandoned_page_purge,
    /// memory purging is delayed by N milli seconds; use 0 for immediate purging or -1 for no purging at all. (=10)
    PurgeDelay = mi_option_e_mi_option_purge_delay,
    /// 0 = use all available numa nodes, otherwise use at most N nodes.
    UseNumaNodes = mi_option_e_mi_option_use_numa_nodes,
    /// If set to 1, do not use OS memory for allocation (but only pre-reserved arenas)
    LimitOsAlloc = mi_option_e_mi_option_limit_os_alloc,
    /// if set, release all memory on exit; sometimes used for dynamic unloading but can be unsafe
    DestroyOnExit = mi_option_e_mi_option_destroy_on_exit,
    /// multiplier for purge_delay for the purging delay for arenas (=10)
    ArenaPurgeMultiplier = mi_option_e_mi_option_arena_purge_mult,
    /// 1 = do not use arena's for allocation (except if using specific arena id's)
    DisallowArenaAllow = mi_option_e_mi_option_disallow_arena_alloc,
    /// allow visiting heap blocks from abandoned threads (=0)
    VisitAbandoned = mi_option_e_mi_option_visit_abandoned,
}

impl MiOptionType {
    pub const fn as_inner(self) -> mi_option_e {
        self as mi_option_e
    }
}

pub struct MiOption;

impl MiOption {
    pub fn disable(opt: MiOptionType) {
        unsafe {
            mi_option_disable(opt.as_inner());
        }
    }

    pub fn enable(opt: MiOptionType) {
        unsafe {
            mi_option_enable(opt.as_inner());
        }
    }

    pub fn get(opt: MiOptionType) -> i64 {
        unsafe { mi_option_get(opt.as_inner()) }
    }

    pub fn get_clamp(opt: MiOptionType, min: i64, max: i64) -> i64 {
        unsafe { mi_option_get_clamp(opt.as_inner(), min, max) }
    }

    pub fn get_size(opt: MiOptionType) -> usize {
        unsafe { mi_option_get_size(opt.as_inner()) }
    }

    pub fn is_enabled(opt: MiOptionType) -> bool {
        unsafe { mi_option_is_enabled(opt.as_inner()) }
    }

    pub fn set(opt: MiOptionType, val: i64) {
        unsafe {
            mi_option_set(opt.as_inner(), val);
        }
    }

    pub fn set_default(opt: MiOptionType, val: i64) {
        unsafe {
            mi_option_set_default(opt.as_inner(), val);
        }
    }

    pub fn set_enabled(opt: MiOptionType, enable: bool) {
        unsafe {
            mi_option_set_enabled(opt.as_inner(), enable);
        }
    }

    pub fn set_enabled_default(opt: MiOptionType, enable: bool) {
        unsafe {
            mi_option_set_enabled_default(opt.as_inner(), enable);
        }
    }
}
