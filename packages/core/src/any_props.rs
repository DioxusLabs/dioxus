use std::marker::PhantomData;

use futures_util::Future;

use crate::{
    factory::{ComponentReturn, RenderReturn},
    innerlude::Scoped,
    scopes::{Scope, ScopeState},
    Element,
};

pub trait AnyProps<'a> {
    fn as_ptr(&self) -> *const ();
    fn render(&'a self, bump: &'a ScopeState) -> RenderReturn<'a>;
    unsafe fn memoize(&self, other: &dyn AnyProps) -> bool;
}

pub(crate) struct VComponentProps<'a, P, A, F = Element<'a>>
where
    F: ComponentReturn<'a, A>,
{
    pub render_fn: fn(Scope<'a, P>) -> F,
    pub memo: unsafe fn(&P, &P) -> bool,
    pub props: *const P,
    pub _marker: PhantomData<A>,
}

impl<'a> VComponentProps<'a, (), ()> {
    pub fn new_empty(render_fn: fn(Scope) -> Element) -> Self {
        Self {
            render_fn,
            memo: <() as PartialEq>::eq,
            props: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }
}

impl<'a, P, A, F: ComponentReturn<'a, A>> VComponentProps<'a, P, A, F> {
    pub(crate) fn new(
        render_fn: fn(Scope<'a, P>) -> F,
        memo: unsafe fn(&P, &P) -> bool,
        props: *const P,
    ) -> Self {
        Self {
            render_fn,
            memo,
            props,
            _marker: PhantomData,
        }
    }
}

impl<'a, P, A, F: ComponentReturn<'a, A>> AnyProps<'a> for VComponentProps<'a, P, A, F> {
    fn as_ptr(&self) -> *const () {
        &self.props as *const _ as *const ()
    }

    // Safety:
    // this will downcast the other ptr as our swallowed type!
    // you *must* make this check *before* calling this method
    // if your functions are not the same, then you will downcast a pointer into a different type (UB)
    unsafe fn memoize(&self, other: &dyn AnyProps) -> bool {
        let real_other: &P = &*(other.as_ptr() as *const _ as *const P);
        let real_us: &P = &*(self.as_ptr() as *const _ as *const P);
        (self.memo)(real_us, real_other)
    }

    fn render(&self, cx: &'a ScopeState) -> RenderReturn<'a> {
        // Make sure the scope ptr is not null
        // self.props.state.set(scope);

        let scope = cx.bump().alloc(Scoped {
            props: unsafe { &*self.props },
            scope: cx,
        });

        // Call the render function directly
        (self.render_fn)(scope).as_return(cx)
    }
}
