use std::{any::Any, marker::PhantomData};

use crate::{ComponentFunction, VirtualDom};

/// A boxed object that can be injected into a component's context.
pub struct BoxedContext(Box<dyn ClonableAny>);

impl BoxedContext {
    /// Create a new boxed context.
    pub fn new(value: impl Any + Clone + 'static) -> Self {
        Self(Box::new(value))
    }

    /// Unwrap the boxed context into its inner value.
    pub fn into_inner(self) -> Box<dyn Any> {
        self.0.into_inner()
    }
}

impl Clone for BoxedContext {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

trait ClonableAny: Any {
    fn clone_box(&self) -> Box<dyn ClonableAny>;

    fn into_inner(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Any + Clone> ClonableAny for T {
    fn clone_box(&self) -> Box<dyn ClonableAny> {
        Box::new(self.clone())
    }

    fn into_inner(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// The platform-independent part of the config needed to launch an application.
pub struct CrossPlatformConfig<
    Component: ComponentFunction<Phantom, Props = Props>,
    Props: Clone + 'static,
    Phantom: 'static,
> {
    /// The root component function.
    pub component: Component,
    /// The props for the root component.
    pub props: Props,
    /// The contexts to provide to the root component.
    pub root_contexts: Vec<BoxedContext>,
    _phantom: PhantomData<Phantom>,
}

impl<
        Component: ComponentFunction<Phantom, Props = Props>,
        Props: Clone + 'static,
        Phantom: 'static,
    > CrossPlatformConfig<Component, Props, Phantom>
{
    /// Create a new cross-platform config.
    pub fn new(component: Component, props: Props, root_contexts: Vec<BoxedContext>) -> Self {
        Self {
            component,
            props,
            root_contexts,
            _phantom: PhantomData,
        }
    }

    /// Build a virtual dom from the config.
    pub fn build_vdom(self) -> VirtualDom {
        let mut vdom = VirtualDom::new_with_props(self.component, self.props);

        for context in self.root_contexts {
            vdom.insert_boxed_root_context(context);
        }

        vdom
    }
}

/// A builder to launch a specific platform.
pub trait PlatformBuilder<Props: Clone + 'static> {
    /// The platform-specific config needed to launch an application.
    type Config: Default;

    /// Launch the app.
    fn launch<Component: ComponentFunction<Phantom, Props = Props>, Phantom: 'static>(
        config: CrossPlatformConfig<Component, Props, Phantom>,
        platform_config: Self::Config,
    );
}

impl<Props: Clone + 'static> PlatformBuilder<Props> for () {
    type Config = ();

    fn launch<Component: ComponentFunction<Phantom, Props = Props>, Phantom: 'static>(
        _: CrossPlatformConfig<Component, Props, Phantom>,
        _: Self::Config,
    ) {
        panic!("No platform is currently enabled. Please enable a platform feature for the dioxus crate.");
    }
}
