#![allow(unused)]
//! Example of components in

use std::{borrow::Borrow, marker::PhantomData};

use dioxus_core::prelude::*;

fn main() {}

static Header: FC<()> = |ctx| {
    let inner = use_ref(ctx, || 0);

    let handler1 = move || println!("Value is {}", inner.borrow());

    ctx.render(dioxus::prelude::LazyNodes::new(|nodectx| {
        builder::ElementBuilder::new(nodectx, "div")
            .child(VNode::Component(VComponent::new(Bottom, (), None)))
            .finish()
    }))
};

static Bottom: FC<()> = |ctx| {
    ctx.render(html! {
        <div>
            <h1> "bruh 1" </h1>
            <h1> "bruh 2" </h1>
        </div>
    })
};

fn Top(ctx: Context<()>) -> VNode {
    ctx.render(html! {
        <div>
            <h1> "bruh 1" </h1>
            <h1> "bruh 2" </h1>
        </div>
    })
}

struct Callback<T>(Box<T>);

// impl<O, T: Fn() -> O> From<T> for Callback<T> {
//     fn from(_: T) -> Self {
//         todo!()
//     }
// }

impl<O, A> From<&dyn Fn(A) -> O> for Callback<&dyn Fn(A) -> O> {
    fn from(_: &dyn Fn(A) -> O) -> Self {
        todo!()
    }
}

impl<O, A, B> From<&dyn Fn(A, B) -> O> for Callback<&dyn Fn(A, B) -> O> {
    fn from(_: &dyn Fn(A, B) -> O) -> Self {
        todo!()
    }
}

// compile time reordering of arguments
// Allows for transparently calling
#[derive(Default)]
pub struct Args<A, B, C> {
    pub a: CuOpt<A>,
    pub b: CuOpt<B>,
    pub c: CuOpt<C>,
}

pub enum CuOpt<T> {
    Some(T),
    None,
}
impl<T> Default for CuOpt<T> {
    fn default() -> Self {
        CuOpt::None
    }
}

impl<T> CuOpt<T> {
    fn unwrap(self) -> T {
        match self {
            CuOpt::Some(t) => t,
            CuOpt::None => panic!(""),
        }
    }
}

trait IsMemo {
    fn memo(&self, other: &Self) -> bool {
        false
    }
}

impl<T: PartialEq> IsMemo for CuOpt<T> {
    fn memo(&self, other: &Self) -> bool {
        self == other
    }
}

impl<T: PartialEq> PartialEq for CuOpt<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CuOpt::Some(a), CuOpt::Some(b)) => a == b,
            (CuOpt::Some(_), CuOpt::None) => false,
            (CuOpt::None, CuOpt::Some(_)) => false,
            (CuOpt::None, CuOpt::None) => true,
        }
    }
}

impl<T> IsMemo for &CuOpt<T> {
    fn memo(&self, other: &Self) -> bool {
        false
    }
}

// #[test]
#[test]
fn try_test() {
    // test_poc()
}

// fn test_poc(ctx: Context) {
//     let b = Bump::new();

//     let h = Args {
//         a: CuOpt::Some("ASD"),
//         b: CuOpt::Some(123),
//         c: CuOpt::Some(|| {}),
//         // c: CuOpt::Some(b.alloc(|| {})),
//         // c: CuOpt::Some(Box::new(|| {}) as Box<dyn Fn()>),
//     };

//     let h2 = Args {
//         a: CuOpt::Some("ASD"),
//         b: CuOpt::Some(123),
//         c: CuOpt::Some(|| {}),
//         // c: CuOpt::Some(b.alloc(|| {})),
//         // c: CuOpt::Some(Box::new(|| {}) as Box<dyn Fn()>),
//         // c: CuOpt::Some(Box::new(|| {}) as Box<dyn Fn()>),
//         // c: CuOpt::Some(Box::new(|| {}) as Box<dyn Fn()>),
//     };

//     // dbg!((&h.a).memo((&&h2.a)));
//     // dbg!((&h.b).memo((&&h2.b)));
//     // dbg!((&h.c).memo((&&h2.c)));
//     //
//     // ctx: Context
//     Top(ctx, &h.a.unwrap(), &h.b.unwrap(), &h.c.unwrap());
// }

// fn test_realzies() {
//     let h = Args {
//         a: CuOpt::Some("ASD"),
//         b: CuOpt::Some(123),
//         c: CuOpt::Some(|| {}),
//     };

//     let g = |ctx: Context| {
//         //
//         Top(ctx, &h.a.unwrap(), &h.b.unwrap(), &h.c.unwrap())
//     };
// }
