//! This file handles the supporting infrastructure for the `Component` trait and `Properties` which makes it possible
//! for components to be used within Nodes.
//!
//! Note - using the builder pattern does not required the Properties trait to be implemented - the only thing that matters is
//! if the type suppports PartialEq. The Properties trait is used by the rsx! and html! macros to generate the type-safe builder
//! that ensures compile-time required and optional fields on props.

use crate::innerlude::FC;

use self::sized_any::SizedAny;

pub type ScopeIdx = generational_arena::Index;

struct ComparableComp<'s> {
    fc_raw: *const (),
    f: &'s dyn SizedAny,
}

impl<'s> ComparableComp<'s> {
    fn compare_to<P: Properties>(&self, other: &ComparableComp) -> bool {
        if self.fc_raw == other.fc_raw {
            let real_other = unsafe { &*(other.f as *const _ as *const P) };
            true
        } else {
            false
        }
    }
}

struct TestProps {}

fn test() {}

mod sized_any {
    use std::any::TypeId;

    // don't allow other implementations of `SizedAny`; `SizedAny` must only be
    // implemented for sized types.
    mod seal {
        // it must be a `pub trait`, but not be reachable - hide it in
        // private mod.
        pub trait Seal {}
    }

    pub trait SizedAny: seal::Seal {}

    impl<T> seal::Seal for T {}
    impl<T> SizedAny for T {}

    // `SizedAny + ?Sized` means it can be a trait object, but `SizedAny` was
    // implemented for the underlying sized type.
    pub fn downcast_ref<From, To>(v: &From) -> Option<&To>
    where
        From: SizedAny + ?Sized + 'static,
        To: 'static,
    {
        // if TypeId::of::<To>() == < From as SizedAny>::get_type_id(v) {
        Some(unsafe { &*(v as *const From as *const To) })
        // } else {
        //     None
        // }
    }
}

pub trait Properties: PartialEq {
    type Builder;
    fn builder() -> Self::Builder;
}

pub struct EmptyBuilder;
impl EmptyBuilder {
    pub fn build(self) -> () {
        ()
    }
}

impl Properties for () {
    type Builder = EmptyBuilder;

    fn builder() -> Self::Builder {
        EmptyBuilder {}
    }
}

pub fn fc_to_builder<T: Properties>(_f: FC<T>) -> T::Builder {
    T::builder()
}
