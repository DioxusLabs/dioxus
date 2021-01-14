mod old {

    // #![feature(type_alias_impl_trait)]
    //
    use std::future::Future;

    trait Props {}
    struct Context<T: Props> {
        _props: std::marker::PhantomData<T>,
    }
    struct VNode {}

    // type FC<T: Props> = fn(&mut Context<T>) -> VNode;
    // type FC<T: Props> = fn(&mut Context<T>) -> Box<dyn Future<Output = VNode>>;

    impl Props for () {}

    // async fn some_component(g: &mut Context<()>) -> VNode {
    //     rsx! {
    //         <div>

    //         </div>
    //     }
    // }
    // Absolve ourselves of any type data about the context itself
    trait ContextApplier {
        fn use_hook<O, H>(
            &mut self,
            initializer: impl FnOnce() -> H,
            runner: impl Fn(&mut H) -> O,
            tear_down: impl Fn(&mut H),
        ) -> O;
    }
    impl<T: Props> ContextApplier for Context<T> {
        fn use_hook<O, H>(
            &mut self,
            initializer: impl FnOnce() -> H,
            runner: impl Fn(&mut H) -> O,
            tear_down: impl Fn(&mut H),
        ) -> O {
            todo!()
        }
    }

    fn use_state<T>(c: &mut impl ContextApplier, g: impl Fn() -> T) -> T {
        c.use_hook(|| {}, |_| {}, |_| {});
        g()
    }

    enum SomeComponent {
        Imperative,
        Async,
    }

    // impl<F, G> From<F> for SomeComponent
    // where
    //     F: Fn() -> G,
    //     G: Future<Output = ()>,
    // {
    //     fn from(_: F) -> Self {
    //         SomeComponent::Async
    //     }
    // }

    // impl From<fn() -> ()> for SomeComponent {
    //     fn from(_: F) -> Self {
    //         SomeComponent::Async
    //     }
    // }
    // impl<F> Into<SomeComponent> for fn() -> F
    // where
    //     F: Future<Output = ()>,
    // {
    //     fn into(self) -> SomeComponent {
    //         todo!()
    //     }
    // }

    // #[test]
    // fn test() {
    //     let b: SomeComponent = test_comp.into();
    // }

    // Does this make sense?
    // Any component labeled with async can halt its rendering, but won't be able to process updates?
    // Or, those updates can still happen virtually, just not propogated into the view?
    // async fn test_comp() -> () {
    //     timer::new(300).await;
    //     html! {
    //         <div>
    //             "hello world!"
    //         </div>
    //     }
    // }

    // fn use_state<T: Props>(c: &mut Context<T>) {}

    // async fn another_component(ctx: &mut Context<()>) -> VNode {
    //     // delay the re-render until component when the future is ready
    //     // "use_future" loads the promise and provides a value (aka a loadable)
    //     let value = use_effect(move || async {
    //         get_value().join(timer::new(300));
    //         set_value(blah);
    //     });

    //     rsx! {
    //         <Suspense fallback={<div>"Loading..."</div>}>
    //             <div>
    //                 "hello {name}!"
    //             </div>
    //         <Suspense />
    //     }
    // }

    /*

    Rationale
    Today, you can do use_async and do some async operations,







    */
    // type FC<P: Props> = fn(&mut Context<P>) -> VNode;

    // static Example: FC<()> = |_| async {
    //     // some async work
    // };

    // type FC2 = fn() -> impl Future<Output = ()>;
    // struct fc<P: Props>(fn(&mut Context<P>) -> G);
    // fn blah<P: Props, G: Future<Output = VNode>>(a: fn(&mut Context<P>) -> G) {}

    // static Example2: FC2<()> = fc(|_| async { VNode {} });
    // static Example2: () = blah(|_: &mut Context<()>| async { VNode {} });

    // static Example: FC<()> = |_| {
    //     let g = async { VNode {} };
    //     Box::new(g)
    // };

    // static Example2:  = || {};

    // type FA<R: Future<Output = i32>> = fn(i32) -> R;

    // async fn my_component()
    // static MyThing: FA<dyn Future<Output = i32>> = |_| async { 10 };

    // type SomeFn = fn() -> ();

    // static MyFn: SomeFn = || {};
}

mod old2 {
    mod vdom {
        //! Virtual DOM implementation
        use super::*;

        pub struct VDom {
            patches: Vec<Patch>,
        }

        impl VDom {
            // fn new(root: ComponentFn) -> Self {
            //     let scope = Scope::new();
            //     Self {}
            // }
        }
    }

    mod nodes {}

    mod patch {}

    mod scope {
        //! Wrappers around components

        pub struct Scope {}

        impl Scope {
            fn new() -> Self {
                Self {}
            }
        }
    }

    mod context {}

    struct EventListener {}

    struct VNode {
        /// key-value pairs of attributes
        attributes: Vec<(&'static str, &'static str)>,

        /// onclick/onhover/on etc listeners
        /// goal is to standardize around a set of cross-platform listeners?
        listeners: Vec<EventListener>,

        /// Direct children, non arena-allocated
        children: Vec<VNode>,
    }

    enum ElementType {
        div,
        p,
        a,
        img,
    }

    struct ComponentContext {}
    type ComponentFn = fn(ctx: &ComponentContext) -> VNode;

    enum Patch {}

    mod tests {
        use super::*;

        /// Ensure components can be made from the raw components
        #[test]
        fn simple_test() {
            fn component(ctx: &ComponentContext) -> VNode {
                println!("Running component");
                VNode {}
            }

            let dom = VDom::new(component);
        }

        /// Ensure components can be made from the raw components
        #[test]
        fn simple_test_closure() {
            let component: ComponentFn = |ctx| {
                println!("Running component");
                VNode {}
            };

            let dom = VDom::new(component);
        }
    }
}
