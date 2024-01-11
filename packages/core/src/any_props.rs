use crate::{nodes::RenderReturn, properties::ComponentFunction};
use std::{any::Any, ops::Deref, panic::AssertUnwindSafe, rc::Rc};

/// A boxed version of AnyProps that can be cloned
pub(crate) struct BoxedAnyProps {
    inner: Box<dyn AnyProps>,
}

impl BoxedAnyProps {
    pub fn new(inner: impl AnyProps + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl Deref for BoxedAnyProps {
    type Target = dyn AnyProps;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl Clone for BoxedAnyProps {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.duplicate(),
        }
    }
}

/// A trait that essentially allows VComponentProps to be used generically
pub(crate) trait AnyProps {
    fn render(&self) -> RenderReturn;
    fn memoize(&self, other: &dyn Any) -> bool;
    fn props(&self) -> &dyn Any;
    fn duplicate(&self) -> Box<dyn AnyProps>;
}

pub(crate) struct VProps<P: 'static, Phantom: 'static> {
    pub render_fn: Rc<dyn ComponentFunction<Phantom, Props = P>>,
    pub memo: fn(&P, &P) -> bool,
    pub props: P,
    pub name: &'static str,
}

impl<P: 'static, Phantom: 'static> VProps<P, Phantom> {
    pub(crate) fn new(
        render_fn: impl ComponentFunction<Phantom, Props = P> + 'static,
        memo: fn(&P, &P) -> bool,
        props: P,
        name: &'static str,
    ) -> Self {
        Self {
            render_fn: Rc::new(render_fn),
            memo,
            props,
            name,
        }
    }
}

impl<P: Clone + 'static, Phantom> AnyProps for VProps<P, Phantom> {
    fn memoize(&self, other: &dyn Any) -> bool {
        match other.downcast_ref::<P>() {
            Some(other) => (self.memo)(&self.props, other),
            None => false,
        }
    }

    fn props(&self) -> &dyn Any {
        &self.props
    }

    fn render(&self) -> RenderReturn {
        let res = std::panic::catch_unwind(AssertUnwindSafe(move || {
            // Call the render function directly
            self.render_fn.call(self.props.clone())
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

    fn duplicate(&self) -> Box<dyn AnyProps> {
        Box::new(Self {
            render_fn: self.render_fn.clone(),
            memo: self.memo,
            props: self.props.clone(),
            name: self.name,
        })
    }
}
