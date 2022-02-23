use std::{cell::RefCell, rc::Rc};

use crate::use_state;
use crate::UseFutureDep;
use dioxus_core::{ScopeState, UiEvent};
use std::future::Future;

pub fn use_callback<I, G: UseFutureDep, F: Future<Output = ()> + 'static>(
    cx: &ScopeState,
    //
    g: G,
    f: impl FnMut(I, G::Out) -> F,
) -> &UseCallback<I, G::Out>
where
    G::Out: 'static,
    I: 'static,
{
    cx.use_hook(|_| {
        //
        UseCallback {
            f: todo!(),
            f2: Box::new(|f| {}),
        }
    })
}

pub struct UseCallback<I, T> {
    f: Rc<RefCell<Option<Box<dyn FnMut(I, T)>>>>,
    f2: Box<dyn Fn(I)>, // f: Rc<RefCell<Option<Box<dyn FnMut(I, T)>>>>,
}

impl<I: 'static, T> std::ops::Deref for UseCallback<I, T> {
    type Target = dyn Fn(I);

    fn deref(&self) -> &Self::Target {
        &self.f2
    }
}

trait MyThing {}
impl<A> MyThing for Box<dyn Fn(A)> {}
impl<A, B> MyThing for Box<dyn Fn(A, B)> {}

#[test]
fn demo() {
    use dioxus_core::prelude::*;
    fn example(cx: Scope) -> Element {
        let (name, _) = use_state(&cx, || 0);
        let (age, _) = use_state(&cx, || 0);

        let onsubmit = use_callback(&cx, (name,), |event: (), (name,)| async move {
            //
        });

        let onsubmit = use_callback(&cx, (name,), my_callback);
        async fn my_callback(event: UiEvent<()>, name: (i32,)) {
            //
        }

        let onsubmit = use_callback(&cx, name, my_callback2);

        async fn my_callback2(event: UiEvent<()>, name: i32) {
            //
        }

        None
    }
}
