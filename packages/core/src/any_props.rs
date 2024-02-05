use crate::{nodes::RenderReturn, ComponentFunction};
use std::{any::Any, panic::AssertUnwindSafe};

pub(crate) type BoxedAnyProps = Box<dyn AnyProps>;

/// A trait for a component that can be rendered.
pub(crate) trait AnyProps: 'static {
    /// Render the component with the internal props.
    fn render(&self) -> RenderReturn;
    /// Check if the props are the same as the type erased props of another component.
    fn memoize(&mut self, other: &dyn Any) -> bool;
    /// Get the props as a type erased `dyn Any`.
    fn props(&self) -> &dyn Any;
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

    fn render(&self) -> RenderReturn {
        let res = std::panic::catch_unwind(AssertUnwindSafe(move || {
            self.render_fn.rebuild(self.props.clone())
        }));

        match res {
            Ok(Some(e)) => RenderReturn::Ready(e),
            Ok(None) => RenderReturn::default(),
            Err(err) => {
                let component_name = self.name;
                tracing::error!("Error while rendering component `{component_name}`: {err:?}");
                RenderReturn::default()
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
