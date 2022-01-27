use std::{borrow::Borrow, cell::Cell, future::Future, rc::Rc};

use dioxus_core::prelude::*;

pub fn use_effect<'a, F, V, O>(cx: &ScopeState, val: &V, f: impl FnOnce() -> F)
where
    F: Future<Output = ()> + 'static,
    V: ToOwned<Owned = O>,
    O: PartialEq + 'static,
{
    let r = val.to_owned();

    let state = cx.use_hook(move |_| UseEffect {
        update: cx.schedule_update(),
        stored: Cell::new(r),
    });

    //
}

struct UseEffect<V: PartialEq> {
    update: Rc<dyn Fn()>,
    stored: Cell<V>,
}

#[test]
fn it_works() {
    // fn app(cx: Scope) -> Element {
    //     let r = "asda".to_string();

    //     use_effect(&cx, r.as_str(), || async {
    //         println!("Hello");
    //     });

    //     todo!()
    // }
}

fn main() {
    let s = String::from("asd");

    let st: &str = s.as_str();

    // let g = st.to_owned();

    // let _ = g == st;

    use_effect2(st, || st.to_owned());

    // use_effect2(st);

    // use_effect2(&(st, st));
}

fn use_effect2<V, O>(val: V, to_own: impl FnOnce() -> O)
where
    O: PartialEq<V> + 'static,
{
    let new_val = val.borrow();
    let r = new_val.to_owned();

    // let _ = &new_val == val;

    // takes_static(new_val);
}

fn takes_static<V: 'static>(v: V) {}
