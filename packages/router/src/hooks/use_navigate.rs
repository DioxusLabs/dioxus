use dioxus_core::ScopeState;
use futures_channel::mpsc::UnboundedSender;
use log::error;

use crate::{
    contexts::RouterContext, navigation::NavigationTarget, service::RouterMessage,
};

/// A hook that allows you to acquire a [`Navigator`] object.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a [`Router`].
/// - Otherwise [`Some`].
///
/// [`Router`]: crate::components::Router
pub fn use_navigate(cx: &ScopeState) -> Option<Navigator> {
    let router = cx.use_hook(|_| {
        let router = cx.consume_context::<RouterContext>();

        // use_navigate only allows to trigger changes and therefore doesn't need to subscribe to
        // updates

        router
    });

    match router {
        Some(router) => Some(Navigator {
            tx: router.tx.clone(),
        }),
        None => {
            error!("`use_navigate` can only be used in descendants of a `Router`");
            None
        }
    }
}

/// A [`Navigator`] allowing for programmatic navigation.
pub struct Navigator {
    tx: UnboundedSender<RouterMessage>,
}

impl Navigator {
    /// Go back to the previous path.
    ///
    /// Will fail silently if there is no path to go back to.
    pub fn go_back(&self) {
        self.tx.unbounded_send(RouterMessage::GoBack).ok();
    }

    /// Go forward to a future path.
    ///
    /// This is the inverse operation of [`Navigator::go_back`]. Will fail silently if there is no
    /// path to go forward to.
    pub fn go_forward(&self) {
        self.tx.unbounded_send(RouterMessage::GoForward).ok();
    }

    /// Push a new path.
    ///
    /// Previous path will be available to go back to.
    pub fn push(&self, target: NavigationTarget) {
        self.tx.unbounded_send(RouterMessage::Push(target)).ok();
    }

    /// Replace the current path.
    ///
    /// Previous path will **not** be available to go back to.
    pub fn replace(&self, target: NavigationTarget) {
        self.tx.unbounded_send(RouterMessage::Replace(target)).ok();
    }
}
