use crate::{
    innerlude::{ComponentPtr, Element, Scope, ScopeId, ScopeState},
    Component,
};
use std::cell::{Cell, RefCell};

/// Virtual Components for custom user-defined components
/// Only supports the functional syntax
pub struct VComponent<'src> {
    /// The key of the component to be used during keyed diffing.
    pub key: Option<&'src str>,

    /// The ID of the component.
    /// Will not be assigned until after the component has been initialized.
    pub scope: Cell<Option<ScopeId>>,

    /// An indication if the component is static (can be memozied)
    pub can_memoize: bool,

    /// The function pointer to the component's render function.
    pub user_fc: ComponentPtr,

    /// The actual name of the component.
    pub fn_name: &'static str,

    /// The props of the component.
    pub props: RefCell<Option<Box<dyn AnyProps + 'src>>>,
}

pub(crate) struct VComponentProps<P> {
    pub render_fn: Component<P>,
    pub memo: unsafe fn(&P, &P) -> bool,
    pub props: P,
}

pub trait AnyProps {
    fn as_ptr(&self) -> *const ();
    fn render<'a>(&'a self, bump: &'a ScopeState) -> Element<'a>;
    unsafe fn memoize(&self, other: &dyn AnyProps) -> bool;
}

impl<P> AnyProps for VComponentProps<P> {
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

    fn render<'a>(&'a self, scope: &'a ScopeState) -> Element<'a> {
        let props = unsafe { std::mem::transmute::<&P, &P>(&self.props) };
        (self.render_fn)(Scope { scope, props })
    }
}
