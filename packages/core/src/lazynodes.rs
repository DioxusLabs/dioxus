use std::marker::PhantomData;

/*
Remember: calls to rsx! are lazy - they are not evaluated immediately.

They also using dynamic dispatch, so we can return multiple rsx!'s from match statements and such.

If we allocated every rsx! call on the heap, it would be quite wasteful. Rsx! calls are FnOnce, so they can be stored in a stack.

Solutions like stackdst are useful, but they only support 'static closures.

All our closures are bound by the bump lifetime, so stack-dst will not work for us

Our solution is to try and manually allocate the closure onto the stack.
If it fails, then we default to Box.

*/
use crate::innerlude::{IntoVNode, NodeFactory, VNode};

/// A concrete type provider for closures that build VNode structures.
///
/// This struct wraps lazy structs that build VNode trees Normally, we cannot perform a blanket implementation over
/// closures, but if we wrap the closure in a concrete type, we can maintain separate implementations of IntoVNode.
///
///
/// ```rust
/// LazyNodes::new(|f| f.element("div", [], [], [] None))
/// ```
// pub struct LazyNodes<'a, F: FnOnce(NodeFactory<'a>) -> VNode<'a>> {
//     inner: Box<F>,
//     _p: PhantomData<&'a ()>,
//     // inner: StackNodeStorage<'a>,
//     // inner: StackNodeStorage<'a>,
// }
// pub type LazyNodes<'b> = Box<dyn for<'a> FnOnce(NodeFactory<'a>) -> VNode<'a> + 'b>;

// pub fn to_lazy_nodes<'b>(
//     f: impl for<'a> FnOnce(NodeFactory<'a>) -> VNode<'a> + 'b,
// ) -> Option<LazyNodes<'b>> {
//     Some(Box::new(f))
// }

type StackHeapSize = [usize; 12];

enum StackNodeStorage<'a> {
    Stack {
        next_ofs: usize,
        buf: StackHeapSize,
        width: usize,
    },
    Heap(Box<dyn FnOnce(NodeFactory<'a>) -> VNode<'a>>),
}

// impl<'a, F: FnOnce(NodeFactory<'a>) -> VNode<'a>> LazyNodes<'a, F> {
//     pub fn new(f: F) -> Self {
//         // let width = std::mem?::size_of::<F>();
//         // let b: Box<dyn FnOnce(NodeFactory<'a>) -> VNode<'a>> = Box::new(f);

//         todo!()
//         // Self { inner: b }
//         // todo!()

//         // if width > std::mem::size_of::<StackHeapSize>() {
//         //     let g: Box<dyn for<'b> FnOnce(NodeFactory<'b>) -> VNode<'b> + 'g> = Box::new(f);
//         //     LazyNodes {
//         //         inner: StackNodeStorage::Heap(g),
//         //     }
//         // } else {
//         //     let mut buf = [0; 12];
//         //     let mut next_ofs = 0;
//         //     next_ofs += 1;
//         //     LazyNodes {
//         //         inner: StackNodeStorage::Stack {
//         //             next_ofs,
//         //             buf,
//         //             width,
//         //         },
//         //     }
//         // }
//     }
// }

// // Our blanket impl
// impl<'a> IntoIterator for LazyNodes<'a>
// // where
// //     F: FnOnce(NodeFactory<'a>) -> VNode<'a>,
// // impl<'a, F> IntoIterator for LazyNodes<'a, F>
// // where
// //     F: FnOnce(NodeFactory<'a>) -> VNode<'a>,
// {
//     type Item = Self;
//     type IntoIter = std::iter::Once<Self::Item>;
//     fn into_iter(self) -> Self::IntoIter {
//         std::iter::once(self)
//     }
// }

// // Our blanket impl
// impl IntoVNode for LazyNodes<'_> {
//     // impl<'a, F: FnOnce(NodeFactory<'a>) -> VNode<'a>> IntoVNode<'a> for LazyNodes<'a, F> {
//     fn into_vnode<'a>(self, cx: NodeFactory<'a>) -> VNode<'a> {
//         todo!()
//         // match self.inner {
//         //     StackNodeStorage::Stack {
//         //         buf,
//         //         next_ofs,
//         //         width,
//         //     } => {
//         //         // get the start of the allocation
//         //         let r = &buf[0];

//         //         // recast the allocation as dyn FnOnce

//         //         // pretend the FnOnce is box
//         //         let g: Box<dyn FnOnce(NodeFactory<'a>) -> VNode<'a>> = todo!();
//         //         // Box::from_raw(r as *const usize as *mut dyn FnOnce(NodeFactory<'a>));

//         //         // use Box's ability to act as FnOnce
//         //         g(cx)
//         //     }
//         //     StackNodeStorage::Heap(b) => b(cx),
//         // }
//     }
// }
