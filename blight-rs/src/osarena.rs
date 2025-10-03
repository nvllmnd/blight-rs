//!
//! Wraps mimalloc arena_id_t api in a safe(ish) Rust data-structure
//!
//!
use core::{
    ffi::c_void,
    ptr::{self, NonNull},
};

use anyhow::{Context, ensure};
use mimalloc_bindgen::api::{mi_arena_area, mi_reserve_os_memory, mi_reserve_os_memory_ex};

use crate::{api::VoidPtr, units::gigabytes};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
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

impl Default for OsArenaOpts {
    fn default() -> Self {
        Self {
            size: gigabytes(2) as usize,
            commit: false,
            exclusive: true,
            allow_large: true,
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OsArena(VoidPtr);

impl OsArena {
    /// Calls mimalloc's mi_reserve_os_memory to have it allocate an initial block of memory
    /// that it will use before calling a system allocator again to request more memory
    /// This wont return an instance of OsArena, as this mimalloc function does not return a
    /// mi_arena_id to associate an arena with.
    ///
    /// returns true on success, or false if out of memory (ENOMEM was returned from inner mimalloc
    /// api call)
    pub fn reserve_os_memory(size_bytes: usize, commit_upfront: bool, allow_large_pages: bool) -> bool {
        let err: i32 = unsafe { mi_reserve_os_memory(size_bytes, commit_upfront, allow_large_pages) };
        err == 0
    }

    pub fn reserve_os_memory_ex(opts: OsArenaOpts) -> anyhow::Result<Self> {
        let mut arena_id: *mut c_void = ptr::null_mut();
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

impl Default for OsArena {
    fn default() -> Self {
        Self::reserve_os_memory_ex(OsArenaOpts::default()).expect("Error creating default OsArena!")
    }
}
