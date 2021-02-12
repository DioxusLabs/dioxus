//! This file handles the supporting infrastructure for the `Component` trait and `Properties` which makes it possible
//! for components to be used within Nodes.
//!

use crate::inner::*;

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
pub trait Properties {
    fn call(&self, ptr: *const ()) {}
}

// Auto implement for no-prop components
impl Properties for () {
    fn call(&self, ptr: *const ()) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::bumpalo::Bump;

    fn test_static_fn<'a, P: Properties>(b: &'a Bump, r: FC<P>) -> VNode<'a> {
        todo!()
    }

    static TestComponent: FC<()> = |ctx, props| {
        //
        ctx.view(html! {
            <div>
            </div>
        })
    };

    static TestComponent2: FC<()> = |ctx, props| {
        //
        ctx.view(|bump: &Bump| VNode::text("blah"))
    };

    #[test]
    fn ensure_types_work() {
        let bump = Bump::new();

        // Happiness! The VNodes are now allocated onto the bump vdom
        let _ = test_static_fn(&bump, TestComponent);
        let _ = test_static_fn(&bump, TestComponent2);
    }
}
