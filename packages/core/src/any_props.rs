use crate::{nodes::RenderReturn, Element};
use std::{ops::Deref, panic::AssertUnwindSafe};

/// A boxed version of AnyProps that can be cloned
pub(crate) struct BoxedAnyProps {
    inner: Box<dyn AnyProps>,
}

impl BoxedAnyProps {
    fn new(inner: impl AnyProps + 'static) -> Self {
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
    fn render<'a>(&'a self) -> RenderReturn;
    fn memoize(&self, other: &dyn AnyProps) -> bool;
    fn duplicate(&self) -> Box<dyn AnyProps>;
}

pub(crate) struct VProps<P> {
    pub render_fn: fn(P) -> Element,
    pub memo: fn(&P, &P) -> bool,
    pub props: P,
}

impl<P> VProps<P> {
    pub(crate) fn new(render_fn: fn(P) -> Element, memo: fn(&P, &P) -> bool, props: P) -> Self {
        Self {
            render_fn,
            memo,
            props,
        }
    }
}

impl<P: Clone> AnyProps for VProps<P> {
    // Safety:
    // this will downcast the other ptr as our swallowed type!
    // you *must* make this check *before* calling this method
    // if your functions are not the same, then you will downcast a pointer into a different type (UB)
    fn memoize(&self, other: &dyn AnyProps) -> bool {
        (self.memo)(self, other)
    }

    fn render(&self) -> RenderReturn {
        let res = std::panic::catch_unwind(AssertUnwindSafe(move || {
            // Call the render function directly
            (self.render_fn)(self.props.clone())
        }));

        match res {
            Ok(Some(e)) => RenderReturn::Ready(e),
            Ok(None) => RenderReturn::default(),
            Err(err) => {
                let component_name = cx.name();
                tracing::error!("Error while rendering component `{component_name}`: {err:?}");
                RenderReturn::default()
            }
        }
    }

    fn duplicate(&self) -> Box<dyn AnyProps> {
        Box::new(Self {
            render_fn: self.render_fn,
            memo: self.memo,
            props: self.props.clone(),
        })
    }
}
