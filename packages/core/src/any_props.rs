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
pub(crate) struct VProps<RenderFn, CompleteBuilder, Props, Marker> {
    render_fn: RenderFn,
    props: CompleteBuilder,
    name: &'static str,
    phantom: std::marker::PhantomData<(Marker, Props)>,
}

impl<RenderFn, CompleteBuilder, Props, Marker> Clone
    for VProps<RenderFn, CompleteBuilder, Props, Marker>
where
    CompleteBuilder: Clone,
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

impl<RenderFn, CompleteBuilder, Props, Marker> VProps<RenderFn, CompleteBuilder, Props, Marker>
where
    CompleteBuilder: Clone,
    Props: Properties<CompleteBuilder>,
{
    /// Create a [`VProps`] object.
    pub fn new(
        render_fn: RenderFn,
        props: CompleteBuilder,
        name: &'static str,
    ) -> VProps<RenderFn, CompleteBuilder, Props, Marker> {
        VProps {
            render_fn,
            props,
            name,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<RenderFn, CompleteBuilder, Props, Marker> AnyPropsBuilder
    for VProps<RenderFn, CompleteBuilder, Props, Marker>
where
    CompleteBuilder: Clone + 'static,
    Props: Properties<CompleteBuilder>,
    RenderFn: ComponentFunction<Props, Marker> + Clone,
    Marker: 'static,
{
    fn create(&self) -> BoxedAnyProps {
        Box::new(
            MountedVProps::<RenderFn, CompleteBuilder, Props, Marker>::new(
                self.render_fn.clone(),
                Props::new(self.props.clone()),
                self.name,
            ),
        )
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
    fn memoize(&mut self, new: &dyn Any) -> bool;
    /// Get the props as a type erased `dyn Any`.
    fn props_mut(&mut self) -> &mut dyn Any;
}

/// A component along with the props the component uses to render.
pub(crate) struct MountedVProps<
    RenderFn,
    CompleteBuilder: Clone,
    Props: Properties<CompleteBuilder>,
    Marker,
> {
    render_fn: RenderFn,
    props: Props::Mounted,
    name: &'static str,
    phantom: std::marker::PhantomData<Marker>,
}

impl<RenderFn, CompleteBuilder, Props, Marker>
    MountedVProps<RenderFn, CompleteBuilder, Props, Marker>
where
    CompleteBuilder: Clone,
    Props: Properties<CompleteBuilder>,
{
    /// Create a [`MountedVProps`] object.
    fn new(
        render_fn: RenderFn,
        props: Props::Mounted,
        name: &'static str,
    ) -> MountedVProps<RenderFn, CompleteBuilder, Props, Marker> {
        MountedVProps {
            render_fn,
            props,
            name,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<RenderFn, CompleteBuilder, Props, Marker> AnyProps
    for MountedVProps<RenderFn, CompleteBuilder, Props, Marker>
where
    CompleteBuilder: Clone + 'static,
    Props: Properties<CompleteBuilder>,
    RenderFn: ComponentFunction<Props, Marker> + Clone,
    Marker: 'static,
{
    fn memoize(&mut self, other: &dyn Any) -> bool {
        match other.downcast_ref::<CompleteBuilder>() {
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
