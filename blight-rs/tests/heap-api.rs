#![feature(allocator_api)]
use std::{alloc::GlobalAlloc, rc::Rc};

use blight_rs::{gpa::MiMallocator, heap::alloc::Heap};

#[global_allocator]
static GLOBAL: MiMallocator = MiMallocator;

#[test]
fn global_allocator_box_and_rc() {
    let b = Box::new(65);
    let rc = Rc::new([0u8; 64]);
    assert!(*b == 65);

    for by in rc.iter() {
        assert!(*by == 0);
    }
}

#[test]
fn heap_alloc_box_and_rc() {
    let h = {
        let h = Heap::default();
        {
            let b = Box::new_in(65, h.as_handle());
            let rc = Rc::new_in([0u8; 64], h.as_handle());
            assert!(*b == 65);

            for by in rc.iter() {
                assert!(*by == 0);
            }
        }
        h
    };

    let mut bx = Box::new_in([0u8; 128], h);
    for (i, bi) in bx.iter_mut().enumerate() {
        *bi = (i * i) as u8;
    }

    for (i, bi) in bx.iter().enumerate() {
        assert!(*bi == (i * i) as u8);
    }
}
