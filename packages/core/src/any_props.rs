use std::{any::Any, panic::AssertUnwindSafe};

use crate::{ComponentFunction, Element, innerlude::CapturedPanic};

pub(crate) type BoxedAnyProps = Box<dyn AnyProps>;

/// Type-erased component props plus the render function that consumes them.
pub(crate) trait AnyProps: 'static {
    /// Render the component with the internal props.
    fn render(&self) -> Element;

    /// Make these props equal to `other`.
    ///
    /// Returns true if the component can be memoized and the render can be skipped.
    fn memoize(&mut self, other: &dyn Any) -> bool;

    /// Return the props as `dyn Any`.
    fn props(&self) -> &dyn Any;

    /// Return the props as mutable `dyn Any`.
    fn props_mut(&mut self) -> &mut dyn Any;

    /// Duplicate this props object for a fresh vnode or scope.
    fn duplicate(&self) -> BoxedAnyProps;
}

/// Component props paired with the component function that renders them.
pub(crate) struct VProps<F: ComponentFunction<P, M>, P, M> {
    render_fn: F,
    memo: fn(&mut P, &P) -> bool,
    props: P,
    name: &'static str,
    phantom: std::marker::PhantomData<M>,
}

impl<F: ComponentFunction<P, M> + Clone, P: Clone + 'static, M: 'static> VProps<F, P, M> {
    pub(crate) fn new(
        render_fn: F,
        memo: fn(&mut P, &P) -> bool,
        props: P,
        name: &'static str,
    ) -> Self {
        Self {
            render_fn,
            memo,
            props,
            name,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<F: ComponentFunction<P, M> + Clone, P: Clone + 'static, M: 'static> AnyProps
    for VProps<F, P, M>
{
    fn render(&self) -> Element {
        fn render_inner(name: &str, res: Result<Element, Box<dyn Any + Send>>) -> Element {
            match res {
                Ok(node) => node,
                Err(err) => {
                    #[cfg(not(target_arch = "wasm32"))]
                    tracing::error!("Panic while rendering component `{name}`: {err:?}");
                    Element::Err(CapturedPanic(err).into())
                }
            }
        }

        render_inner(
            self.name,
            std::panic::catch_unwind(AssertUnwindSafe(|| {
                self.render_fn.rebuild(self.props.clone())
            })),
        )
    }

    fn memoize(&mut self, other: &dyn Any) -> bool {
        match other.downcast_ref::<P>() {
            Some(other) => (self.memo)(&mut self.props, other),
            None => false,
        }
    }

    fn props(&self) -> &dyn Any {
        &self.props
    }

    fn props_mut(&mut self) -> &mut dyn Any {
        &mut self.props
    }

    fn duplicate(&self) -> BoxedAnyProps {
        Box::new(Self {
            render_fn: self.render_fn.clone(),
            memo: self.memo,
            props: self.props.clone(),
            name: self.name,
            phantom: std::marker::PhantomData,
        })
    }
}
