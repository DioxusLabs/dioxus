use std::marker::PhantomData;

use crate::{
    factory::{ComponentReturn, RenderReturn},
    innerlude::Scoped,
    scopes::{Scope, ScopeState},
    Element,
};

/// A trait that essentially allows VComponentProps to be used generically
pub unsafe trait AnyProps<'a> {
    fn props_ptr(&self) -> *const ();
    fn render(&'a self, bump: &'a ScopeState) -> RenderReturn<'a>;
    unsafe fn memoize(&self, other: &dyn AnyProps) -> bool;
}

pub(crate) struct VProps<'a, P, A, F: ComponentReturn<'a, A> = Element<'a>> {
    pub render_fn: fn(Scope<'a, P>) -> F,
    pub memo: unsafe fn(&P, &P) -> bool,
    pub props: P,
    // pub props: PropsAllocation<P>,
    _marker: PhantomData<A>,
}

impl<'a, P, A, F> VProps<'a, P, A, F>
where
    F: ComponentReturn<'a, A>,
{
    pub(crate) fn new(
        render_fn: fn(Scope<'a, P>) -> F,
        memo: unsafe fn(&P, &P) -> bool,
        props: P,
    ) -> Self {
        Self {
            render_fn,
            memo,
            props,
            // props: PropsAllocation::Borrowed(props),
            _marker: PhantomData,
        }
    }
}

unsafe impl<'a, P, A, F> AnyProps<'a> for VProps<'a, P, A, F>
where
    F: ComponentReturn<'a, A>,
{
    fn props_ptr(&self) -> *const () {
        &self.props as *const _ as *const ()
    }

    // Safety:
    // this will downcast the other ptr as our swallowed type!
    // you *must* make this check *before* calling this method
    // if your functions are not the same, then you will downcast a pointer into a different type (UB)
    unsafe fn memoize(&self, other: &dyn AnyProps) -> bool {
        let real_other: &P = &*(other.props_ptr() as *const _ as *const P);
        let real_us: &P = &*(self.props_ptr() as *const _ as *const P);
        (self.memo)(real_us, real_other)
    }

    fn render(&'a self, cx: &'a ScopeState) -> RenderReturn<'a> {
        let scope = cx.bump().alloc(Scoped {
            props: &self.props,
            scope: cx,
        });

        // Call the render function directly
        (self.render_fn)(scope).as_return(cx)
    }
}
