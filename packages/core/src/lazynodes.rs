//! Support for storing lazy-nodes on the stack
//!
//! This module provides support for a type called `LazyNodes` which is a micro-heap located on the stack to make calls
//! to `rsx!` more efficient.
//!
//! To support returning rsx! from branches in match statements, we need to use dynamic dispatch on NodeFactory closures.
//!
//! This can be done either through boxing directly, or by using dynamic-sized-types and a custom allocator. In our case,
//! we build a tiny alloactor in the stack and allocate the closure into that.
//!
//! The logic for this was borrowed from <https://docs.rs/stack_dst/0.6.1/stack_dst/>. Unfortunately, this crate does not
//! support non-static closures, so we've implemented the core logic of `ValueA` in this module.

use crate::innerlude::{NodeFactory, VNode};
use std::mem;

/// A concrete type provider for closures that build VNode structures.
///
/// This struct wraps lazy structs that build VNode trees Normally, we cannot perform a blanket implementation over
/// closures, but if we wrap the closure in a concrete type, we can maintain separate implementations of IntoVNode.
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
    Heap(Box<dyn FnMut(Option<NodeFactory<'a>>) -> Option<VNode<'a>> + 'b>),
}

impl<'a, 'b> LazyNodes<'a, 'b> {
    pub fn new_some<F>(_val: F) -> Self
    where
        F: FnOnce(NodeFactory<'a>) -> VNode<'a> + 'b,
    {
        Self::new(_val)
    }

    /// force this call onto the stack
    pub fn new_boxed<F>(_val: F) -> Self
    where
        F: FnOnce(NodeFactory<'a>) -> VNode<'a> + 'b,
    {
        // there's no way to call FnOnce without a box, so we need to store it in a slot and use static dispatch
        let mut slot = Some(_val);

        let val = move |fac: Option<NodeFactory<'a>>| {
            let inner = slot.take().unwrap();
            fac.map(inner)
        };

        Self {
            inner: StackNodeStorage::Heap(Box::new(val)),
        }
    }

    pub fn new(_val: impl FnOnce(NodeFactory<'a>) -> VNode<'a> + 'b) -> Self {
        // there's no way to call FnOnce without a box, so we need to store it in a slot and use static dispatch
        let mut slot = Some(_val);

        let val = move |fac: Option<NodeFactory<'a>>| {
            let inner = slot.take().unwrap();
            fac.map(inner)
        };

        // miri does not know how to work with mucking directly into bytes
        if cfg!(miri) {
            Self {
                inner: StackNodeStorage::Heap(Box::new(val)),
            }
        } else {
            unsafe { LazyNodes::new_inner(val) }
        }
    }

    unsafe fn new_inner<F>(val: F) -> Self
    where
        F: FnMut(Option<NodeFactory<'a>>) -> Option<VNode<'a>> + 'b,
    {
        let mut ptr: *const _ = &val as &dyn FnMut(Option<NodeFactory<'a>>) -> Option<VNode<'a>>;

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
            let dataptr = buf.as_mut()[..].as_mut_ptr() as *mut u8;
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

    pub fn call(self, f: NodeFactory<'a>) -> VNode<'a> {
        match self.inner {
            StackNodeStorage::Heap(mut lazy) => lazy(Some(f)).unwrap(),
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
    fn call<'a>(&mut self, f: NodeFactory<'a>) -> VNode<'a> {
        let LazyStack { buf, .. } = self;
        let data = buf.as_ref();

        let info_size =
            mem::size_of::<*mut dyn FnMut(Option<NodeFactory<'a>>) -> Option<VNode<'a>>>()
                / mem::size_of::<usize>()
                - 1;

        let info_ofs = data.len() - info_size;

        let g: *mut dyn FnMut(Option<NodeFactory<'a>>) -> Option<VNode<'a>> =
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

            let info_size = mem::size_of::<
                *mut dyn FnMut(Option<NodeFactory<'_>>) -> Option<VNode<'_>>,
            >() / mem::size_of::<usize>()
                - 1;

            let info_ofs = data.len() - info_size;

            let g: *mut dyn FnMut(Option<NodeFactory<'_>>) -> Option<VNode<'_>> =
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::innerlude::{Element, Scope, VirtualDom};

    #[test]
    fn it_works() {
        fn app(cx: Scope<()>) -> Element {
            cx.render(LazyNodes::new_some(|f| {
                f.text(format_args!("hello world!"))
            }))
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild();

        let g = dom.base_scope().root_node();
        dbg!(g);
    }

    #[test]
    fn it_drops() {
        use std::rc::Rc;

        struct AppProps {
            inner: Rc<i32>,
        }

        fn app(cx: Scope<AppProps>) -> Element {
            struct DropInner {
                id: i32,
            }
            impl Drop for DropInner {
                fn drop(&mut self) {
                    log::debug!("dropping inner");
                }
            }

            let caller = {
                let it = (0..10)
                    .map(|i| {
                        let val = cx.props.inner.clone();

                        LazyNodes::new_some(move |f| {
                            log::debug!("hell closure");
                            let inner = DropInner { id: i };
                            f.text(format_args!("hello world {:?}, {:?}", inner.id, val))
                        })
                    })
                    .collect::<Vec<_>>();

                LazyNodes::new_some(|f| {
                    log::debug!("main closure");
                    f.fragment_from_iter(it)
                })
            };

            cx.render(caller)
        }

        let inner = Rc::new(0);
        let mut dom = VirtualDom::new_with_props(
            app,
            AppProps {
                inner: inner.clone(),
            },
        );
        dom.rebuild();

        drop(dom);

        assert_eq!(Rc::strong_count(&inner), 1);
    }
}
