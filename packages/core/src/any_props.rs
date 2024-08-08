use crate::{innerlude::CapturedPanic, nodes::RenderReturn, ComponentFunction};
use std::{any::Any, panic::AssertUnwindSafe};

pub(crate) type BoxedAnyProps = Box<dyn AnyProps>;

/// A trait for a component that can be rendered.
pub(crate) trait AnyProps: 'static {
    /// Render the component with the internal props.
    fn render(&self) -> RenderReturn;
    /// Make the old props equal to the new type erased props. Return if the props were equal and should be memoized.
    fn memoize(&mut self, other: &dyn Any) -> bool;
    /// Get the props as a type erased `dyn Any`.
    fn props(&self) -> &dyn Any;
    /// Get the props as a type erased `dyn Any`.
    fn props_mut(&mut self) -> &mut dyn Any;
    /// Duplicate this component into a new boxed component.
    fn duplicate(&self) -> BoxedAnyProps;
}

/// A component along with the props the component uses to render.
pub(crate) struct VProps<F: ComponentFunction<P, M>, P, M> {
    render_fn: F,
    memo: fn(&mut P, &P) -> bool,
    props: P,
    name: &'static str,
    phantom: std::marker::PhantomData<M>,
}

impl<F: ComponentFunction<P, M>, P: Clone, M> Clone for VProps<F, P, M> {
    fn clone(&self) -> Self {
        Self {
            render_fn: self.render_fn.clone(),
            memo: self.memo,
            props: self.props.clone(),
            name: self.name,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<F: ComponentFunction<P, M> + Clone, P: Clone + 'static, M: 'static> VProps<F, P, M> {
    /// Create a [`VProps`] object.
    pub fn new(
        render_fn: F,
        memo: fn(&mut P, &P) -> bool,
        props: P,
        name: &'static str,
    ) -> VProps<F, P, M> {
        VProps {
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

    fn render(&self) -> RenderReturn {
        let res = std::panic::catch_unwind(AssertUnwindSafe(move || {
            self.render_fn.rebuild(self.props.clone())
        }));

        match res {
            Ok(node) => RenderReturn { node },
            Err(err) => {
                let component_name = self.name;
                tracing::error!("Panic while rendering component `{component_name}`: {err:?}");
                let panic = CapturedPanic { error: err };
                RenderReturn {
                    node: Err(panic.into()),
                }
            }
        }
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
