use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use futures_channel::mpsc::UnboundedSender;

use crate::{navigation::NamedNavigationSegment, service::RouterMessage, state::CurrentRoute};

/// A context providing read and write access to the [router service](crate::service::RouterService).
#[derive(Clone)]
pub(crate) struct RouterContext {
    /// A sender to send messages to the [router service](crate::service::RouterService).
    pub(crate) tx: UnboundedSender<RouterMessage>,
    /// A shared memory space the [router service](crate::service::RouterService) uses
    /// to provide information on the current route.
    pub(crate) state: Arc<RwLock<CurrentRoute>>,
    /// The named routes the router knows about.
    pub(crate) named_routes: Arc<BTreeMap<&'static str, Vec<NamedNavigationSegment>>>,
}
