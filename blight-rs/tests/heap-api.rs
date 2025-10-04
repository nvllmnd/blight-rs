#![feature(allocator_api)]

use blight_rs::{gpa::MiMallocator, heap::alloc::Heap};
use std::rc::Rc;

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

#[test]
fn global_strings_and_formating() {
    let mut s = String::new();
    s.push_str("hello mayteee");

    println!("{}", s);

    let x = 59;
    let y = x.to_string();

    assert_eq!("hello mayteee", s);
    assert_eq!("59", y);
}
