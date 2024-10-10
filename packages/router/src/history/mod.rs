//! History Integration
//!
//! dioxus-router relies on the [Document](dioxus_lib::document::Document) to store the current Route, and possibly a
//! history (i.e. a browsers back button) and future (i.e. a browsers forward button).
//!
//! To integrate dioxus-router with a any type of history, all you have to do is implement the
//! [`dioxus_lib::document::Document`] trait.

use std::{any::Any, rc::Rc, sync::Arc};

pub(crate) trait AnyHistoryProvider {
    fn parse_route(&self, route: &str) -> Result<Rc<dyn Any>, String>;

    #[must_use]
    fn current_route(&self) -> Rc<dyn Any>;

    #[must_use]
    fn can_go_back(&self) -> bool {
        true
    }

    fn go_back(&mut self);

    #[must_use]
    fn can_go_forward(&self) -> bool {
        true
    }

    fn go_forward(&mut self);

    fn push(&mut self, route: Rc<dyn Any>);

    fn replace(&mut self, path: Rc<dyn Any>);

    #[allow(unused_variables)]
    fn external(&mut self, url: String) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {}

    #[cfg(feature = "liveview")]
    fn is_liveview(&self) -> bool;
}
