#![no_std]
#![feature(allocator_api)]

extern crate alloc;

pub mod arena;
pub mod gpa;
pub mod mimalloc;
pub mod temp;

#[cfg(test)]
mod test {

    use core::{ffi::c_void, ptr::NonNull};

    use mimalloc_bindgen::api::{mi_free, mi_malloc_aligned};

    use super::*;

    #[test]
    fn alloc_and_free() {
        let p = unsafe { mi_malloc_aligned(8, 8) } as *mut u8;
        let p = NonNull::new(p).expect("mi_malloc_aligned(8,8) returned nullptr!");
        unsafe { NonNull::write(p, 255) };
        assert!(unsafe { NonNull::read(p) } == 255);
        unsafe { mi_free(p.cast::<c_void>().as_ptr()) }
    }
}
