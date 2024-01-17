use crate::{nodes::RenderReturn, Component};
use std::{any::Any, panic::AssertUnwindSafe};

pub type BoxedAnyProps = Box<dyn AnyProps>;

/// A trait that essentially allows VComponentProps to be used generically
pub(crate) trait AnyProps {
    fn render(&self) -> RenderReturn;
    fn memoize(&self, other: &dyn Any) -> bool;
    fn props(&self) -> &dyn Any;
    fn duplicate(&self) -> BoxedAnyProps;
}

pub(crate) struct VProps<P: 'static> {
    pub render_fn: Component<P>,
    pub memo: fn(&P, &P) -> bool,
    pub props: P,
    pub name: &'static str,
}

impl<P: 'static> VProps<P> {
    pub(crate) fn new(
        render_fn: Component<P>,
        memo: fn(&P, &P) -> bool,
        props: P,
        name: &'static str,
    ) -> Self {
        Self {
            render_fn,
            memo,
            props,
            name,
        }
    }
}

impl<P: Clone + 'static> AnyProps for VProps<P> {
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
            (self.render_fn)(self.props.clone())
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
        })
    }
}
