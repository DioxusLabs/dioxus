// #![allow(unused, non_upper_case_globals)]
// use bumpalo::Bump;
// use dioxus_core::nodebuilder::*;
// use dioxus_core::prelude::VNode;
// use dioxus_core::prelude::*;
// use once_cell::sync::{Lazy, OnceCell};

use std::ops::Deref;

/*
A guard over underlying T that provides access in callbacks via "Copy"
*/

// #[derive(Clone)]
struct ContextGuard2<T> {
    _val: std::marker::PhantomData<T>,
}
impl<T> Clone for ContextGuard2<T> {
    // we aren't cloning the underlying data so clone isn't necessary
    fn clone(&self) -> Self {
        todo!()
    }
}
impl<T> Copy for ContextGuard2<T> {}

impl<T> ContextGuard2<T> {
    fn get<'a>(&'a self) -> ContextLock<'a, T> {
        todo!()
    }
}

struct ContextLock<'a, T> {
    _val: std::marker::PhantomData<&'a T>,
}
impl<'a, T: 'a + 'static> Deref for ContextLock<'a, T> {
    type Target = T;

    fn deref<'b>(&'b self) -> &'b T {
        todo!()
    }
}

/*
The source of the data that gives out context guards
*/
struct Context<'a> {
    _p: std::marker::PhantomData<&'a ()>,
}

impl<'a> Context<'a> {
    fn use_context<'b, I, O: 'b>(&self, _f: fn(&'b I) -> O) -> ContextGuard2<O> {
        todo!()
    }
    fn add_listener(&self, _f: impl Fn(()) + 'a) {
        todo!()
    }

    fn render(self, _f: impl FnOnce(&'a String) + 'a) {}
    // fn view(self, f: impl for<'b> FnOnce(&'a String) + 'a) {}
    // fn view(self, f: impl for<'b> FnOnce(&'b String) + 'a) {}
}

struct Example {
    value: String,
}
/*
Example compiling
*/
fn t<'a>(ctx: Context<'a>) {
    let value = ctx.use_context(|b: &Example| &b.value);

    // Works properly, value is moved by copy into the closure
    let refed = value.get();
    println!("Value is {}", refed.as_str());
    let r2 = refed.as_str();

    ctx.add_listener(move |_| {
        // let val = value.get().as_str();
        let _val2 = r2.as_bytes();
        println!("v2 is {}", r2);
        // println!("refed is {}", refed);
    });

    // let refed = value.deref();
    // returns &String

    // returns &String
    // let refed = value.deref(); // returns &String
    // let refed = value.deref(); // returns &String

    // Why does this work? This closure should be static but is holding a reference to refed
    // The context guard is meant to prevent any references moving into the closure
    // if the references move they might become invalid due to mutlithreading issues
    ctx.add_listener(move |_| {
        // let val = value.as_str();
        // let val2 = refed.as_bytes();
    });

    ctx.render(move |_b| {});
}

fn main() {}
