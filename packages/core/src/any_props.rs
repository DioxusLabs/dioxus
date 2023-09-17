use crate::{
    innerlude::Scoped,
    nodes::RenderReturn,
    scopes::{Scope, ScopeState},
    Element,
};
use std::panic::AssertUnwindSafe;

/// A trait that essentially allows VComponentProps to be used generically
///
/// # Safety
///
/// This should not be implemented outside this module
pub(crate) unsafe trait AnyProps<'a> {
    fn props_ptr(&self) -> *const ();
    fn render(&'a self, bump: &'a ScopeState) -> RenderReturn<'a>;
    unsafe fn memoize(&self, other: &dyn AnyProps) -> bool;
}

pub(crate) struct VProps<'a, P> {
    pub render_fn: fn(Scope<'a, P>) -> Element<'a>,
    pub memo: unsafe fn(&P, &P) -> bool,
    pub props: P,
}

impl<'a, P> VProps<'a, P> {
    pub(crate) fn new(
        render_fn: fn(Scope<'a, P>) -> Element<'a>,
        memo: unsafe fn(&P, &P) -> bool,
        props: P,
    ) -> Self {
        Self {
            render_fn,
            memo,
            props,
        }
    }
}

unsafe impl<'a, P> AnyProps<'a> for VProps<'a, P> {
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
        let res = std::panic::catch_unwind(AssertUnwindSafe(move || {
            // Call the render function directly
            let scope: &mut Scoped<P> = cx.bump().alloc(Scoped {
                props: &self.props,
                scope: cx,
            });

            (self.render_fn)(scope)
        }));

        match res {
            Ok(Some(e)) => RenderReturn::Ready(e),
            Ok(None) => RenderReturn::default(),
            Err(err) => {
                let component_name = cx.name();
                tracing::error!("Error while rendering component `{component_name}`: {err:?}");
                RenderReturn::default()
            }
        }
    }
}
