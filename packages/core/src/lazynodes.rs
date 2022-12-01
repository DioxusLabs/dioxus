//! Support for storing lazy-nodes on the stack
//!
//! This module provides support for a type called `LazyNodes` which is a micro-heap located on the stack to make calls
//! to `rsx!` more efficient.
//!
//! To support returning rsx! from branches in match statements, we need to use dynamic dispatch on [`ScopeState`] closures.
//!
//! This can be done either through boxing directly, or by using dynamic-sized-types and a custom allocator. In our case,
//! we build a tiny alloactor in the stack and allocate the closure into that.
//!
//! The logic for this was borrowed from <https://docs.rs/stack_dst/0.6.1/stack_dst/>. Unfortunately, this crate does not
//! support non-static closures, so we've implemented the core logic of `ValueA` in this module.

use crate::{innerlude::VNode, ScopeState};
use std::mem;

/// A concrete type provider for closures that build [`VNode`] structures.
///
/// This struct wraps lazy structs that build [`VNode`] trees Normally, we cannot perform a blanket implementation over
/// closures, but if we wrap the closure in a concrete type, we can maintain separate implementations of [`IntoVNode`].
///
///
/// ```rust, ignore
/// LazyNodes::new(|f| f.element("div", [], [], [] None))
/// ```
pub struct LazyNodes<'a, 'b> {
    inner: StackNodeStorage<'a, 'b>,
}

type StackHeapSize = [usize; 16];

enum StackNodeStorage<'a, 'b> {
    Stack(LazyStack),
    Heap(Box<dyn FnMut(Option<&'a ScopeState>) -> Option<VNode<'a>> + 'b>),
}

impl<'a, 'b> LazyNodes<'a, 'b> {
    /// Create a new [`LazyNodes`] closure, optimistically placing it onto the stack.
    ///
    /// If the closure cannot fit into the stack allocation (16 bytes), then it
    /// is placed on the heap. Most closures will fit into the stack, and is
    /// the most optimal way to use the creation function.
    pub fn new(val: impl FnOnce(&'a ScopeState) -> VNode<'a> + 'b) -> Self {
        // there's no way to call FnOnce without a box, so we need to store it in a slot and use static dispatch
        let mut slot = Some(val);

        let val = move |fac: Option<&'a ScopeState>| {
            fac.map(
                slot.take()
                    .expect("LazyNodes closure to be called only once"),
            )
        };

        // miri does not know how to work with mucking directly into bytes
        // just use a heap allocated type when miri is running
        if cfg!(miri) {
            Self {
                inner: StackNodeStorage::Heap(Box::new(val)),
            }
        } else {
            unsafe { LazyNodes::new_inner(val) }
        }
    }

    /// Create a new [`LazyNodes`] closure, but force it onto the heap.
    pub fn new_boxed<F>(inner: F) -> Self
    where
        F: FnOnce(&'a ScopeState) -> VNode<'a> + 'b,
    {
        // there's no way to call FnOnce without a box, so we need to store it in a slot and use static dispatch
        let mut slot = Some(inner);

        Self {
            inner: StackNodeStorage::Heap(Box::new(move |fac: Option<&'a ScopeState>| {
                fac.map(
                    slot.take()
                        .expect("LazyNodes closure to be called only once"),
                )
            })),
        }
    }

    unsafe fn new_inner<F>(val: F) -> Self
    where
        F: FnMut(Option<&'a ScopeState>) -> Option<VNode<'a>> + 'b,
    {
        let mut ptr: *const _ = &val as &dyn FnMut(Option<&'a ScopeState>) -> Option<VNode<'a>>;

        assert_eq!(
            ptr as *const u8, &val as *const _ as *const u8,
            "MISUSE: Closure returned different pointer"
        );
        assert_eq!(
            std::mem::size_of_val(&*ptr),
            std::mem::size_of::<F>(),
            "MISUSE: Closure returned a subset pointer"
        );

        let words = ptr_as_slice(&mut ptr);
        assert!(
            words[0] == &val as *const _ as usize,
            "BUG: Pointer layout is not (data_ptr, info...)"
        );

        // - Ensure that Self is aligned same as data requires
        assert!(
            std::mem::align_of::<F>() <= std::mem::align_of::<Self>(),
            "TODO: Enforce alignment >{} (requires {})",
            std::mem::align_of::<Self>(),
            std::mem::align_of::<F>()
        );

        let info = &words[1..];
        let data = words[0] as *mut ();
        let size = mem::size_of::<F>();

        let stored_size = info.len() * mem::size_of::<usize>() + size;
        let max_size = mem::size_of::<StackHeapSize>();

        if stored_size > max_size {
            Self {
                inner: StackNodeStorage::Heap(Box::new(val)),
            }
        } else {
            let mut buf: StackHeapSize = StackHeapSize::default();

            assert!(info.len() + round_to_words(size) <= buf.as_ref().len());

            // Place pointer information at the end of the region
            // - Allows the data to be at the start for alignment purposes
            {
                let info_ofs = buf.as_ref().len() - info.len();
                let info_dst = &mut buf.as_mut()[info_ofs..];
                for (d, v) in Iterator::zip(info_dst.iter_mut(), info.iter()) {
                    *d = *v;
                }
            }

            let src_ptr = data as *const u8;
            let dataptr = buf.as_mut_ptr().cast::<u8>();

            for i in 0..size {
                *dataptr.add(i) = *src_ptr.add(i);
            }

            std::mem::forget(val);

            Self {
                inner: StackNodeStorage::Stack(LazyStack {
                    _align: [],
                    buf,
                    dropped: false,
                }),
            }
        }
    }

    /// Call the closure with the given factory to produce real [`VNode`].
    ///
    /// ```rust, ignore
    /// let f = LazyNodes::new(move |f| f.element("div", [], [], [] None));
    ///
    /// let node = f.call(cac);
    /// ```
    #[must_use]
    pub fn call(self, f: &'a ScopeState) -> VNode<'a> {
        match self.inner {
            StackNodeStorage::Heap(mut lazy) => {
                lazy(Some(f)).expect("Closure should not be called twice")
            }
            StackNodeStorage::Stack(mut stack) => stack.call(f),
        }
    }
}

struct LazyStack {
    _align: [u64; 0],
    buf: StackHeapSize,
    dropped: bool,
}

impl LazyStack {
    fn call<'a>(&mut self, f: &'a ScopeState) -> VNode<'a> {
        let LazyStack { buf, .. } = self;
        let data = buf.as_ref();

        let info_size =
            mem::size_of::<*mut dyn FnMut(Option<&'a ScopeState>) -> Option<VNode<'a>>>()
                / mem::size_of::<usize>()
                - 1;

        let info_ofs = data.len() - info_size;

        let g: *mut dyn FnMut(Option<&'a ScopeState>) -> Option<VNode<'a>> =
            unsafe { make_fat_ptr(data[..].as_ptr() as usize, &data[info_ofs..]) };

        self.dropped = true;

        let clos = unsafe { &mut *g };
        clos(Some(f)).unwrap()
    }
}
impl Drop for LazyStack {
    fn drop(&mut self) {
        if !self.dropped {
            let LazyStack { buf, .. } = self;
            let data = buf.as_ref();

            let info_size =
                mem::size_of::<*mut dyn FnMut(Option<&ScopeState>) -> Option<VNode<'_>>>()
                    / mem::size_of::<usize>()
                    - 1;

            let info_ofs = data.len() - info_size;

            let g: *mut dyn FnMut(Option<&ScopeState>) -> Option<VNode<'_>> =
                unsafe { make_fat_ptr(data[..].as_ptr() as usize, &data[info_ofs..]) };

            self.dropped = true;

            let clos = unsafe { &mut *g };
            clos(None);
        }
    }
}

/// Obtain mutable access to a pointer's words
fn ptr_as_slice<T>(ptr: &mut T) -> &mut [usize] {
    assert!(mem::size_of::<T>() % mem::size_of::<usize>() == 0);
    let words = mem::size_of::<T>() / mem::size_of::<usize>();
    // SAFE: Points to valid memory (a raw pointer)
    unsafe { core::slice::from_raw_parts_mut(ptr as *mut _ as *mut usize, words) }
}

/// Re-construct a fat pointer
unsafe fn make_fat_ptr<T: ?Sized>(data_ptr: usize, meta_vals: &[usize]) -> *mut T {
    let mut rv = mem::MaybeUninit::<*mut T>::uninit();
    {
        let s = ptr_as_slice(&mut rv);
        s[0] = data_ptr;
        s[1..].copy_from_slice(meta_vals);
    }
    let rv = rv.assume_init();
    assert_eq!(rv as *const (), data_ptr as *const ());
    rv
}

fn round_to_words(len: usize) -> usize {
    (len + mem::size_of::<usize>() - 1) / mem::size_of::<usize>()
}
