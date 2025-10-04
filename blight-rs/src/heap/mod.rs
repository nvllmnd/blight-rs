use core::{marker::PhantomData, ptr::NonNull};

use mimalloc_bindgen::api::mi_heap_t;

use crate::heap::alloc::Heap;

type MiHeapPtr = NonNull<mi_heap_t>;

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InnerHeap(MiHeapPtr);

pub mod alloc;
pub mod handle;
mod inner;
