#![allow(missing_docs)]
use super::UseFutureDep;
use dioxus_core::{ScopeState, TaskId};
use futures_util::{future, Stream, StreamExt};
use std::{cell::Cell, rc::Rc, sync::Arc};

/// A stream that calls the provided callback when an item is available.
///
/// This runs through the stream only once - though the stream may be regenerated
/// through the [`UseStream::restart`] method.
///
/// This is commonly used for components that needs to update their values from stream items.
///
/// Whenever the hooks dependencies change, the stream will be re-evaluated.
/// If a stream is pending when the dependencies change, the previous stream
/// will be dropped before the new one is started.
///
/// - dependencies: a tuple of references to values that are PartialEq + Clone
///
/// ```ignore
/// use dioxus::prelude::*;
/// use futures_util::{stream, StreamExt};
/// use std::time::Duration;
///
/// const Example: Component = |cx| {
///     let count = use_state(cx, || 0);
///     let make_stream = |_| {
///         stream::iter(1..).then(|second| async move {
///             gloo_timers::future::sleep(Duration::from_secs(1)).await;
///             second
///         })
///     };
///     let on_item = {
///         let count = count.clone();
///         move |second| count.set(second)
///     };
///     use_stream(cx, (), make_stream, on_item);
///
///     cx.render(rsx! { div { "seconds: {count}" } })
/// }
/// ```
pub fn use_stream<T, S, D>(
    cx: &ScopeState,
    dependencies: D,
    stream: impl FnOnce(D::Out) -> S,
    on_item: impl Fn(S::Item) + 'static,
) -> &UseStream
where
    T: 'static,
    S: Stream<Item = T> + 'static,
    D: UseFutureDep,
{
    let state = cx.use_hook(move || UseStream {
        update: cx.schedule_update(),
        needs_regen: Rc::new(Cell::new(true)),
        task: Default::default(),
    });

    let state_dependencies = cx.use_hook(Vec::new);

    if dependencies.clone().apply(state_dependencies) || state.needs_regen.get() {
        // kill the old one, if it exists
        if let Some(task) = state.task.take() {
            cx.remove_future(task);
        }

        // Create the new stream
        let stream = stream(dependencies.out());
        let task = state.task.clone();

        state.task.set(Some(cx.push_future(async move {
            stream
                .for_each(|value| {
                    on_item(value);
                    future::ready(())
                })
                .await;
            task.take();
        })));

        // Mark that we don't need to regenerate
        state.needs_regen.set(false);
    }

    state
}

pub enum StreamState<'a, T> {
    Pending,
    Complete(&'a T),
    Regenerating(&'a T), // the old value
}

#[derive(Clone)]
pub struct UseStream {
    update: Arc<dyn Fn()>,
    needs_regen: Rc<Cell<bool>>,
    task: Rc<Cell<Option<TaskId>>>,
}

impl UseStream {
    /// Restart the stream with new dependencies.
///
    /// Will not cancel the previous stream, but will ignore any values that it
    /// generates.
    pub fn restart(&self) {
        self.needs_regen.set(true);
        (self.update)();
    }

    /// Forcefully drop a stream
    pub fn cancel(&self, cx: &ScopeState) {
        if let Some(task) = self.task.take() {
            cx.remove_future(task);
        }
    }

    /// Get the ID of the future in Dioxus' internal scheduler
    pub fn task(&self) -> Option<TaskId> {
        self.task.get()
    }
}

/// A helper macro that merges uses the closure syntax to elaborate the dependency array
#[macro_export]
macro_rules! use_stream {
    ($cx:ident, || $($rest:tt)*) => { use_stream( $cx, (), |_| $($rest)* ) };
    ($cx:ident, | $($args:tt),* | $($rest:tt)*) => {
        use_stream(
            $cx,
            ($($args),*),
            |($($args),*)| $($rest)*
        )
    };
}

#[cfg(test)]
mod tests {
    use futures_util::{future, stream};

    use super::*;

    #[allow(unused)]
    #[test]
    fn test_use_stream() {
        use dioxus_core::prelude::*;

        struct MyProps {
            a: String,
            b: i32,
            c: i32,
            d: i32,
            e: i32,
        }

        async fn app(cx: Scope<'_, MyProps>) -> Element {
            // should only ever run once
            use_stream(cx, (), |_| stream::once(future::ready(())), |_| {});

            // runs when a is changed
            use_stream(
                cx,
                (&cx.props.a,),
                |(a,)| stream::once(future::ready(())),
                |_| (),
            );

            // runs when a or b is changed
            use_stream(
                cx,
                (&cx.props.a, &cx.props.b),
                |(a, b)| stream::once(future::ready(123)),
                |_: i32| (),
            );

            let a = use_stream!(cx, || stream::once(future::ready(())), |_| {});

            let b = &123;
            let c = &123;

            let a = use_stream!(
                cx,
                |b, c| stream::once(async move {
                    let a = b + c;
                    let blah = "asd";
                    blah
                }),
                |_: &str| {}
            );

            todo!()
        }
    }
}
