use dioxus::prelude::*;
use futures_channel::mpsc::UnboundedSender;
use log::error;

use crate::{contexts::RouterContext, navigation::NavigationTarget, service::RouterMessage};

/// A hook that allows you to acquire a [`Navigator`] object.
///
/// # Return values
/// - [`None`], when the calling component is not nested within a [`Router`].
/// - Otherwise [`Some`].
///
/// # Panic
/// - When the calling component is not nested within a [`Router`], but only in debug builds.
///
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// fn SomeComponent(cx: Scope) -> Element {
///     let nav = use_navigate(&cx).expect("router as ancestor");
///
///     # let some_condition = true;
///     if some_condition {
///         nav.push("https://dioxuslabs.com/");
///     }
///
///     cx.render(rsx! {
///         p { "content" }
///     })
/// }
/// ```
///
/// [`Router`]: crate::components::Router
#[must_use]
pub fn use_navigate(cx: &ScopeState) -> Option<Navigator> {
    // use_navigate doesn't provide access to router state and therefore doesn't need to subscribe
    // for updates
    let router = cx.use_hook(|| cx.consume_context::<RouterContext>());

    match router {
        Some(router) => Some(Navigator {
            tx: router.tx.clone(),
        }),
        None => {
            error!("`use_navigate` can only be used in descendants of a `Router`");
            #[cfg(debug_assertions)]
            panic!("`use_navigate` can only be used in descendants of a `Router`");
            #[cfg(not(debug_assertions))]
            None
        }
    }
}

/// A [`Navigator`] allowing for programmatic navigation.
///
/// A [`Navigator`] is not guaranteed to be able to trigger navigation. For example, it will not be
/// able to do so, when the [`Router`](crate::components::Router) is `init_only`.
#[derive(Debug)]
pub struct Navigator {
    tx: UnboundedSender<RouterMessage>,
}

impl Navigator {
    /// Go back to the previous path.
    ///
    /// Will fail silently if there is no path to go back to.
    pub fn go_back(&self) {
        let _ = self.tx.unbounded_send(RouterMessage::GoBack);
    }

    /// Go forward to a future path.
    ///
    /// This is the inverse operation of [`Navigator::go_back`]. Will fail silently if there is no
    /// path to go forward to.
    pub fn go_forward(&self) {
        let _ = self.tx.unbounded_send(RouterMessage::GoForward);
    }

    /// Push a new path.
    ///
    /// Previous path will be available to go back to.
    pub fn push(&self, target: impl Into<NavigationTarget>) {
        let _ = self.tx.unbounded_send(RouterMessage::Push(target.into()));
    }

    /// Replace the current path.
    ///
    /// Previous path will **not** be available to go back to.
    pub fn replace(&self, target: impl Into<NavigationTarget>) {
        let _ = self
            .tx
            .unbounded_send(RouterMessage::Replace(target.into()));
    }
}

#[cfg(test)]
mod tests {
    use futures_channel::mpsc::{unbounded, UnboundedReceiver};

    use super::*;
    use NavigationTarget::InternalTarget;

    #[test]
    fn go_back() {
        let (n, mut rx) = prepare();
        n.go_back();

        assert!(matches!(rx.try_next(), Ok(Some(RouterMessage::GoBack))));
    }

    #[test]
    fn go_forward() {
        let (n, mut rx) = prepare();
        n.go_forward();

        assert!(matches!(rx.try_next(), Ok(Some(RouterMessage::GoForward))));
    }

    #[test]
    fn push() {
        let (n, mut rx) = prepare();
        n.push(InternalTarget(String::from("path")));

        assert!(matches!(
            rx.try_next(),
            Ok(Some(RouterMessage::Push(InternalTarget(_))))
        ));
    }

    #[test]
    fn replace() {
        let (n, mut rx) = prepare();
        n.replace(InternalTarget(String::from("path")));

        assert!(matches!(
            rx.try_next(),
            Ok(Some(RouterMessage::Replace(InternalTarget(_))))
        ));
    }

    fn prepare() -> (Navigator, UnboundedReceiver<RouterMessage>) {
        let (tx, rx) = unbounded();
        (Navigator { tx }, rx)
    }
}
