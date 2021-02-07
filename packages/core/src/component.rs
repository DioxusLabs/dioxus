use crate::inner::*;
use crate::prelude::bumpalo::Bump;

/// The `Component` trait refers to any struct or funciton that can be used as a component
/// We automatically implement Component for FC<T>
pub trait Component {
    type Props: Properties;
    fn builder(&'static self) -> Self::Props;
}

// Auto implement component for a FC
// Calling the FC is the same as "rendering" it
impl<P: Properties> Component for FC<P> {
    type Props = P;

    fn builder(&self) -> Self::Props {
        todo!()
    }
}

/// The `Properties` trait defines any struct that can be constructed using a combination of default / optional fields.
/// Components take a "properties" object
pub trait Properties: 'static {
    fn new() -> Self;
}

// Auto implement for no-prop components
impl Properties for () {
    fn new() -> Self {
        ()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_static_fn<'a, P: Properties>(b: &'a Bump, r: FC<P>) -> VNode<'a> {
        todo!()
    }

    fn test_component(ctx: Context<()>) -> VNode {
        ctx.view(html! {<div> </div> })
    }

    fn test_component2(ctx: Context<()>) -> VNode {
        ctx.view(|bump: &Bump| VNode::text("blah"))
    }

    #[test]
    fn ensure_types_work() {
        let bump = Bump::new();

        // Happiness! The VNodes are now allocated onto the bump vdom
        let _ = test_static_fn(&bump, test_component);
        let _ = test_static_fn(&bump, test_component2);
    }
}
