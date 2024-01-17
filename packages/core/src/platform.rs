use std::any::Any;

use crate::{
    any_props::{AnyProps, VProps},
    properties::ComponentFunction,
    VirtualDom,
};

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
#[derive(Clone)]
pub struct CrossPlatformConfig<P: AnyProps> {
    /// The root component function.
    props: P,
    // /// The contexts to provide to the root component.
    // root_contexts: Vec<BoxedContext>,
}

impl<P: AnyProps> CrossPlatformConfig<P> {}

impl<P: AnyProps> CrossPlatformConfig<P> {
    /// Create a new cross-platform config.
    pub fn new(props: P) -> Self {
        CrossPlatformConfig {
            props,
            // root_contexts,
        }
    }

    // /// Push a new context into the root component's context.
    // pub fn push_context<T: Any + Clone + 'static>(&mut self, context: T) {
    //     self.root_contexts.push(BoxedContext::new(context));
    // }

    /// Build a virtual dom from the config.
    pub fn build_vdom(self) -> VirtualDom {
        let mut vdom = VirtualDom::new_with_component(self.props);

        // for context in self.root_contexts {
        //     vdom.insert_boxed_root_context(context);
        // }

        vdom
    }
}

/// A builder to launch a specific platform.
pub trait PlatformBuilder<P: AnyProps> {
    /// The platform-specific config needed to launch an application.
    type Config: Default;

    /// Launch the app.
    fn launch(config: CrossPlatformConfig<P>, platform_config: Self::Config);
}

impl<P: AnyProps> PlatformBuilder<P> for () {
    type Config = ();

    fn launch(_: CrossPlatformConfig<P>, _: Self::Config) {
        panic!("No platform is currently enabled. Please enable a platform feature for the dioxus crate.");
    }
}
