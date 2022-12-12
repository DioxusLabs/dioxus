use futures_channel::mpsc::UnboundedSender;

use crate::{navigation::NavigationTarget, RouterMessage};

/// A [`Navigator`] allowing for programmatic navigation.
///
/// The [`Navigator`] is not guaranteed to be able to trigger a navigation. When and if a navigation
/// is actually handled depends on the UI library.
pub struct Navigator<I> {
    sender: UnboundedSender<RouterMessage<I>>,
}

impl<I> Navigator<I> {
    /// Go back to the previous location.
    ///
    /// Will fail silently if there is no previous location to go to.
    pub fn go_back(&self) {
        let _ = self.sender.unbounded_send(RouterMessage::GoBack);
    }

    /// Go back to the next location.
    ///
    /// Will fail silently if there is no next location to go to.
    pub fn go_forward(&self) {
        let _ = self.sender.unbounded_send(RouterMessage::GoForward);
    }

    /// Push a new location.
    ///
    /// The previous location will be available to go back to.
    pub fn push(&self, target: impl Into<NavigationTarget>) {
        let _ = self
            .sender
            .unbounded_send(RouterMessage::Push(target.into()));
    }

    /// Replace the current location.
    ///
    /// The previous location will **not** be available to go back to.
    pub fn replace(&self, target: impl Into<NavigationTarget>) {
        let _ = self
            .sender
            .unbounded_send(RouterMessage::Replace(target.into()));
    }
}

impl<I> From<UnboundedSender<RouterMessage<I>>> for Navigator<I> {
    fn from(sender: UnboundedSender<RouterMessage<I>>) -> Self {
        Self { sender }
    }
}

#[cfg(test)]
mod tests {
    use futures_channel::mpsc::{unbounded, UnboundedReceiver};

    use super::*;

    fn prepare() -> (Navigator<()>, UnboundedReceiver<RouterMessage<()>>) {
        let (sender, receiver) = unbounded();
        (Navigator::from(sender), receiver)
    }

    #[test]
    fn go_back() {
        let (n, mut s) = prepare();
        n.go_back();

        assert_eq!(s.try_next().unwrap(), Some(RouterMessage::GoBack));
    }

    #[test]
    fn go_forward() {
        let (n, mut s) = prepare();
        n.go_forward();

        assert_eq!(s.try_next().unwrap(), Some(RouterMessage::GoForward));
    }

    #[test]
    fn push() {
        let (n, mut s) = prepare();
        let target = NavigationTarget::from("https://dioxuslabs.com/");
        n.push(target.clone());

        assert_eq!(s.try_next().unwrap(), Some(RouterMessage::Push(target)));
    }

    #[test]
    fn replace() {
        let (n, mut s) = prepare();
        let target = NavigationTarget::from("https://dioxuslabs.com/");
        n.replace(target.clone());

        assert_eq!(s.try_next().unwrap(), Some(RouterMessage::Replace(target)));
    }
}
