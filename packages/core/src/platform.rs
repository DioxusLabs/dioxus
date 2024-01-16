use std::any::Any;

use crate::{properties::ComponentFn, Component, VirtualDom};

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
pub struct CrossPlatformConfig<Props: Clone + 'static> {
    /// The root component function.
    pub component: Component<Props>,
    /// The props for the root component.
    pub props: Props,
    /// The contexts to provide to the root component.
    pub root_contexts: Vec<BoxedContext>,
}

impl<Props: Clone + 'static> CrossPlatformConfig<Props> {
    /// Create a new cross-platform config.
    pub fn new<M>(
        component: impl ComponentFn<Props, M>,
        props: Props,
        root_contexts: Vec<BoxedContext>,
    ) -> Self {
        Self {
            component: ComponentFn::as_component(std::rc::Rc::new(component)),
            props,
            root_contexts,
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
    fn launch(config: CrossPlatformConfig<Props>, platform_config: Self::Config);
}

impl<Props: Clone + 'static> PlatformBuilder<Props> for () {
    type Config = ();

    fn launch(_: CrossPlatformConfig<Props>, _: Self::Config) {
        panic!("No platform is currently enabled. Please enable a platform feature for the dioxus crate.");
    }
}
