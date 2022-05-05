use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use futures_channel::mpsc::UnboundedSender;

use crate::{navigation::NamedNavigationSegment, service::RouterMessage, state::RouterState};

/// A context providing read and write access to the [router service].
///
/// [router service]: crate::service::RouterService
#[derive(Clone)]
pub(crate) struct RouterContext {
    /// A class to apply to active links.
    pub(crate) active_class: Option<String>,
    /// A sender to send messages to the [router service].
    ///
    /// [router service]: crate::service::RouterService
    pub(crate) tx: UnboundedSender<RouterMessage>,
    /// Shared memory the [router service] uses to provide information on the current route.
    ///
    /// [router service]: crate::service::RouterService
    pub(crate) state: Arc<RwLock<RouterState>>,
    /// The named routes the router knows about.
    pub(crate) named_routes: Arc<BTreeMap<&'static str, Vec<NamedNavigationSegment>>>,
}
