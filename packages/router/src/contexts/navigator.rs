use crate::prelude::{ExternalNavigationFailure, IntoRoutable, RouterContext};

/// Acquire the navigator without subscribing to updates.
///
/// Can be called anywhere in the application provided a Router has been initialized.
///
/// ## Panics
///
/// Panics if there is no router present.
pub fn navigator() -> Navigator {
    Navigator(
        dioxus_lib::prelude::try_consume_context::<RouterContext>()
            .expect("A router must be present to use navigator"),
    )
}

/// A view into the navigation state of a router.
#[derive(Clone, Copy)]
pub struct Navigator(pub(crate) RouterContext);

impl Navigator {
    /// Check whether there is a previous page to navigate back to.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.0.can_go_back()
    }

    /// Check whether there is a future page to navigate forward to.
    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.0.can_go_forward()
    }

    /// Go back to the previous location.
    ///
    /// Will fail silently if there is no previous location to go to.
    pub fn go_back(&self) {
        self.0.go_back();
    }

    /// Go back to the next location.
    ///
    /// Will fail silently if there is no next location to go to.
    pub fn go_forward(&self) {
        self.0.go_forward();
    }

    /// Push a new location.
    ///
    /// The previous location will be available to go back to.
    pub fn push(&self, target: impl Into<IntoRoutable>) -> Option<ExternalNavigationFailure> {
        self.0.push(target)
    }

    /// Replace the current location.
    ///
    /// The previous location will **not** be available to go back to.
    pub fn replace(&self, target: impl Into<IntoRoutable>) -> Option<ExternalNavigationFailure> {
        self.0.replace(target)
    }
}
