use crate::{innerlude::CapturedPanic, nodes::RenderReturn, ComponentFunction, Properties};
use std::{any::Any, panic::AssertUnwindSafe};

pub(crate) type BoxedAnyPropsBuilder = Box<dyn AnyPropsBuilder>;

/// A trait for a builder that can be mounted to a scope
pub(crate) trait AnyPropsBuilder: 'static {
    /// Create a new [`AnyProps`] object.
    fn create(&self) -> BoxedAnyProps;
    /// Duplicate this component into a new boxed component.
    fn duplicate(&self) -> BoxedAnyPropsBuilder;
    /// Return the props builder
    fn props(&self) -> &dyn Any;
}

/// A component along with the props the component uses to render.
pub(crate) struct VProps<RenderFn, Props: Properties, Marker> {
    render_fn: RenderFn,
    props: Props::CompleteBuilder,
    name: &'static str,
    phantom: std::marker::PhantomData<Marker>,
}

impl<RenderFn, Props: Properties, Marker> Clone for VProps<RenderFn, Props, Marker>
where
    RenderFn: Clone,
{
    fn clone(&self) -> Self {
        Self {
            render_fn: self.render_fn.clone(),
            props: self.props.clone(),
            name: self.name,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<RenderFn, Props: Properties, Marker> VProps<RenderFn, Props, Marker> {
    /// Create a [`VProps`] object.
    pub fn new(
        render_fn: RenderFn,
        props: Props::CompleteBuilder,
        name: &'static str,
    ) -> VProps<RenderFn, Props, Marker> {
        VProps {
            render_fn,
            props,
            name,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<RenderFn, Props, Marker> AnyPropsBuilder for VProps<RenderFn, Props, Marker>
where
    Props: Properties,
    RenderFn: ComponentFunction<Props, Marker> + Clone,
    Marker: 'static,
{
    fn create(&self) -> BoxedAnyProps {
        Box::new(MountedVProps::<RenderFn, Props, Marker>::new(
            self.render_fn.clone(),
            Props::new(self.props.clone()),
            self.name,
        ))
    }

    fn duplicate(&self) -> BoxedAnyPropsBuilder {
        Box::new(self.clone())
    }

    fn props(&self) -> &dyn Any {
        &self.props
    }
}

pub(crate) type BoxedAnyProps = Box<dyn AnyProps>;

/// A trait for a component that can be rendered.
pub(crate) trait AnyProps: 'static {
    /// Render the component with the internal props.
    fn render(&self) -> RenderReturn;
    /// Make the old props equal to the new type erased props. Return if the props were equal and should be memoized.
    fn memoize(&mut self, other: &dyn Any) -> bool;
    /// Get the props as a type erased `dyn Any`.
    fn props_mut(&mut self) -> &mut dyn Any;
}

/// A component along with the props the component uses to render.
pub(crate) struct MountedVProps<RenderFn, Props: Properties, Marker> {
    render_fn: RenderFn,
    props: Props::Mounted,
    name: &'static str,
    phantom: std::marker::PhantomData<Marker>,
}

impl<RenderFn, Props: Properties, Marker> MountedVProps<RenderFn, Props, Marker> {
    /// Create a [`MountedVProps`] object.
    fn new(
        render_fn: RenderFn,
        props: Props::Mounted,
        name: &'static str,
    ) -> MountedVProps<RenderFn, Props, Marker> {
        MountedVProps {
            render_fn,
            props,
            name,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<RenderFn, Props: Properties, Marker> AnyProps for MountedVProps<RenderFn, Props, Marker>
where
    RenderFn: ComponentFunction<Props, Marker> + Clone,
    Props: Properties,
    Marker: 'static,
{
    fn memoize(&mut self, other: &dyn Any) -> bool {
        match other.downcast_ref::<Props::CompleteBuilder>() {
            Some(other) => Props::memoize(&mut self.props, other),
            None => false,
        }
    }

    fn props_mut(&mut self) -> &mut dyn Any {
        &mut self.props
    }

    fn render(&self) -> RenderReturn {
        let res = std::panic::catch_unwind(AssertUnwindSafe(move || {
            self.render_fn.rebuild(Props::props(&self.props))
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
}
