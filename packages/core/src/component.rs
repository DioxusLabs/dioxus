//! This file handles the supporting infrastructure for the `Component` trait and `Properties` which makes it possible
//! for components to be used within Nodes.
//!
//! Note - using the builder pattern does not required the Properties trait to be implemented - the only thing that matters is
//! if the type suppports PartialEq. The Properties trait is used by the rsx! and html! macros to generate the type-safe builder
//! that ensures compile-time required and optional fields on ctx.

use crate::innerlude::FC;

pub type ScopeIdx = generational_arena::Index;

pub unsafe trait Properties: PartialEq + Sized {
    type Builder;
    const CAN_BE_MEMOIZED: bool;
    fn builder() -> Self::Builder;
}

pub struct EmptyBuilder;
impl EmptyBuilder {
    pub fn build(self) -> () {
        ()
    }
}

unsafe impl Properties for () {
    const CAN_BE_MEMOIZED: bool = true;
    type Builder = EmptyBuilder;

    fn builder() -> Self::Builder {
        EmptyBuilder {}
    }
}

pub fn fc_to_builder<T: Properties>(_f: FC<T>) -> T::Builder {
    T::builder()
}

mod testing {
    use std::{any::Any, ops::Deref};

    use crate::innerlude::VNode;

    // trait PossibleProps {
    //     type POut: PartialEq;
    //     fn as_partial_eq(&self) -> Option<&Self::POut> {
    //         None
    //     }
    // }

    // impl<T: PartialEq> PossibleProps for T {
    //     type POut = Self;
    // }

    // struct SomeProps2<'a> {
    //     inner: &'a str,
    // }

    // Fallback trait for to all types to default to `false`.
    trait NotEq {
        const IS_EQ: bool = false;
    }
    impl<T> NotEq for T {}

    // Concrete wrapper type where `IS_COPY` becomes `true` if `T: Copy`.
    struct IsEq<G, T>(std::marker::PhantomData<(G, T)>);

    impl<G: PartialEq, T: PartialEq<G>> IsEq<G, T> {
        // Because this is implemented directly on `IsCopy`, it has priority over
        // the `NotCopy` trait impl.
        //
        // Note: this is a *totally different* associated constant from that in
        // `NotCopy`. This does not specialize the `NotCopy` trait impl on `IsCopy`.
        const IS_EQ: bool = true;
    }

    #[derive(PartialEq)]
    struct SomeProps {
        inner: &'static str,
    }

    struct SomeProps2 {
        inner: &'static str,
    }

    #[test]
    fn test() {
        let g = IsEq::<SomeProps, SomeProps>::IS_EQ;

        // let g = IsEq::<Vec<u32>>::IS_COPY;
        // let g = IsEq::<u32>::IS_COPY;
        // dbg!(g);

        // let props = SomeProps { inner: "asd" };

        // let intermediate: Box<dyn PartialEq<SomeProps>> = Box::new(props);
        // let as_any: Box<dyn Any> = Box::new(intermediate);

        // let as_partialeq = as_any
        //     .downcast_ref::<Box<dyn PartialEq<SomeProps>>>()
        //     .unwrap();
    }

    // struct blah {}
    // #[reorder_args]
    pub fn blah(a: i32, b: &str, c: &str) {}

    // pub mod blah {
    //     pub const a: u8 = 0;
    //     pub const b: u8 = 1;
    // }

    trait Eat {}
    impl Eat for fn() {}
    impl<T> Eat for fn(T) {}
    impl<T, K> Eat for fn(T, K) {}

    mod other {
        use super::blah;
        fn test2() {
            // rsx!{
            //     div {
            //         Ele {
            //             a: 10,
            //             b: "asd"
            //             c: impl Fn() -> ()
            //         }
            //     }
            // }

            // becomes

            // const reorder: fn(_, _) = |a, b| {};
            // blah::META;
            // let a = 10;
            // let b = "asd";
            // let g = [10, 10.0];
            // let c = g.a;

            // blah(10, "asd");
        }
    }

    struct Inner<'a> {
        a: String,
        b: i32,
        c: &'a str,
    }

    struct Custom<'a, P: 'a> {
        inner: &'a P,
        // inner: *const (),
        _p: std::marker::PhantomData<&'a P>,
    }

    impl<'a, P> Custom<'a, P> {
        fn props(&self) -> &P {
            todo!()
        }
    }

    // impl<P> Deref for Custom<P> {
    //     type Target = Inner;

    //     fn deref(&self) -> &Self::Target {
    //         unsafe { &*self.inner }
    //     }
    // }

    fn test2<'a>(a: Custom<'a, Inner<'a>>) -> VNode {
        let r = a.inner;

        todo!()
        // let g = a.props();
        // todo!()
        // let g = &a.a;
    }

    fn is_comp() {}
}

mod style {}
