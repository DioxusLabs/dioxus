use crate::{nodes::RenderReturn, ComponentFunction};
use std::{any::Any, panic::AssertUnwindSafe};

pub(crate) type BoxedAnyProps = Box<dyn AnyProps>;

/// A trait that essentially allows VComponentProps to be used generically
pub(crate) trait AnyProps {
    fn render(&self) -> RenderReturn;
    fn memoize(&self, other: &dyn Any) -> bool;
    fn props(&self) -> &dyn Any;
    fn duplicate(&self) -> BoxedAnyProps;
}

/// Create a new boxed props object.
pub(crate) fn new_any_props<F: ComponentFunction<P, M>, P: Clone + 'static, M: 'static>(
    render_fn: F,
    memo: fn(&P, &P) -> bool,
    props: P,
    name: &'static str,
) -> Box<dyn AnyProps> {
    Box::new(VProps {
        render_fn,
        memo,
        props,
        name,
        phantom: std::marker::PhantomData,
    })
}

struct VProps<F: ComponentFunction<P, M>, P, M> {
    render_fn: F,
    memo: fn(&P, &P) -> bool,
    props: P,
    name: &'static str,
    phantom: std::marker::PhantomData<M>,
}

impl<F: ComponentFunction<P, M> + Clone, P: Clone + 'static, M: 'static> AnyProps
    for VProps<F, P, M>
{
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
            self.render_fn.rebuild(self.props.clone())
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
            phantom: std::marker::PhantomData,
        })
    }
}
