
mod use2 {
    #![allow(missing_docs)]

    use dioxus_core::{ScopeState, TaskId};
    use std::{
        any::Any,
        cell::{Cell, RefCell},
        future::Future,
        rc::Rc,
    };

    /// A future that resolves to a value.
    ///
    /// This runs the future only once - though the future may be regenerated
    /// through the [`UseFuture::restart`] method.
    ///
    /// This is commonly used for components that cannot be rendered until some
    /// asynchronous operation has completed.
    ///
    ///
    ///
    ///
    ///
    pub fn use_future<'a>(
        // pub fn use_future<'a, T: 'static, F: Future<Output = T> + 'static>(
        cx: &'a ScopeState,
    ) -> &'a UseFuture<()> {
        //     let state = cx.use_hook(move |_| UseFuture {
        //         update: cx.schedule_update(),
        //         needs_regen: Cell::new(true),
        //         slot: Rc::new(Cell::new(None)),
        //         value: None,
        //         task: None,
        //         pending: true,
        //         dep_cont: Cell::new(0),
        //         deps: RefCell::new(Vec::new()),
        //         first_time: true,
        //     });

        //     if let Some(value) = state.slot.take() {
        //         state.value = Some(value);
        //         state.task = None;
        //     }

        //     if state.needs_regen.get() {
        //         // We don't need regen anymore
        //         state.needs_regen.set(false);
        //         state.pending = false;

        //         // Create the new future
        //         let fut = new_fut();

        //         // Clone in our cells
        //         let slot = state.slot.clone();
        //         let updater = state.update.clone();

        //         state.task = Some(cx.push_future(async move {
        //             let res = fut.await;
        //             slot.set(Some(res));
        //             updater();
        //         }));
        //     }

        //     state.first_time = false;

        //     state

        // new_fut: impl FnOnce() -> F,

        todo!()
    }

    pub enum FutureState<'a, T> {
        Pending,
        Complete(&'a T),
        Regenerating(&'a T), // the old value
    }

    pub struct UseFuture<T> {
        update: Rc<dyn Fn()>,
        needs_regen: Cell<bool>,
        value: Option<T>,
        slot: Rc<Cell<Option<T>>>,
        task: Option<TaskId>,
        pending: bool,
        deps: RefCell<Vec<Box<dyn Any>>>,
        dep_cont: Cell<usize>,
        first_time: bool,
    }

    impl<T> UseFuture<T> {
        pub fn restart(&self) {
            self.needs_regen.set(true);
            (self.update)();
        }

        // clears the value in the future slot without starting the future over
        pub fn clear(&self) -> Option<T> {
            (self.update)();
            self.slot.replace(None)
        }

        // Manually set the value in the future slot without starting the future over
        pub fn set(&self, new_value: T) {
            self.slot.set(Some(new_value));
            self.needs_regen.set(true);
            (self.update)();
        }

        pub fn value(&self) -> Option<&T> {
            self.value.as_ref()
        }

        pub fn state(&self) -> FutureState<T> {
            // self.value.as_ref()
            FutureState::Pending
        }

        pub fn build<F>(&self, new_fut: impl FnOnce() -> F) {}
    }

    #[test]
    fn test_use_future_deps() {
        use dioxus_core::prelude::*;

        struct MyProps {
            val: String,
            name: i32,
        }

        fn app(cx: Scope<MyProps>) -> Element {
            let MyProps { val, name } = cx.props;

            // let val = use_c(&cx)
            //     .use_dep(val)
            //     .restart_if(|| false)
            //     .use_dep(name)
            //     .build(|(val, name)| async move {});

            // async fn fetch_thing(name: String, num: i32) -> String {
            //     format!("{} {}", name, num)
            // }

            // let val = use_future(&cx, || fetch_thing(val.clone(), *name))
            //     .with_dep(val)
            //     .with_dep(name)
            //     .restart_if(|| false);

            None
        }
    }

    // pub struct CoroutineBuilder<'a, const PARAM: usize> {
    //     deps: Vec<Box<dyn Any>>,
    //     cx: &'a ScopeState,
    // }

    // macro_rules! dep_impl {
    //     ($id1:literal to $id2:literal) => {
    //         impl<'a> CoroutineBuilder<'a, $id1> {
    //             pub fn use_dep<F: 'static + PartialEq + Clone>(
    //                 mut self,
    //                 dep: &F,
    //             ) -> CoroutineBuilder<'a, $id2> {
    //                 self.deps.push(Box::new(dep.clone()));
    //                 unsafe { std::mem::transmute(self) }
    //             }
    //         }
    //     };
    // }

    // dep_impl!(0 to 1);
    // dep_impl!(1 to 2);
    // dep_impl!(2 to 3);
    // dep_impl!(3 to 4);
    // dep_impl!(4 to 5);
    // dep_impl!(5 to 6);
    // dep_impl!(6 to 7);
    // dep_impl!(7 to 8);
    // dep_impl!(8 to 9);
}
