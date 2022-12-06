use async_rwlock::RwLockReadGuard;
use dioxus::prelude::*;
use dioxus_router_core::{
    history::{HistoryProvider, MemoryHistory},
    routes::{ContentAtom, Segment},
    RouterService, RouterState, RoutingCallback,
};

use crate::contexts::router::RouterContext;

pub fn use_router<'a>(
    cx: &'a ScopeState,
    cfg: &dyn Fn() -> RouterConfiguration,
    content: &dyn Fn() -> Segment<Component>,
) -> (RwLockReadGuard<'a, RouterState<Component>>, ()) {
    let (service, state, sender) = cx.use_hook(|| {
        let cfg = cfg();
        let content = content();

        let (mut service, sender, state) = RouterService::new(
            content,
            cfg.history,
            cx.schedule_update_any(),
            cfg.on_update,
            cfg.failure_external_navigation,
            cfg.failure_named_navigation,
            cfg.failure_redirection_limit,
        );

        cx.provide_context(RouterContext {
            state: state.clone(),
            sender: sender.clone(),
        });

        (
            if cfg.synchronous {
                Some(service)
            } else {
                cx.spawn(async move { service.run().await });
                None
            },
            state,
            sender,
        )
    });

    if let Some(service) = service {
        service.run_current();
    }

    (
        loop {
            if let Some(state) = state.try_read() {
                break state;
            }
        },
        (),
    )
}

pub struct RouterConfiguration {
    pub failure_external_navigation: ContentAtom<Component>,
    pub failure_named_navigation: ContentAtom<Component>,
    pub failure_redirection_limit: ContentAtom<Component>,
    pub history: Box<dyn HistoryProvider>,
    pub on_update: Option<RoutingCallback<Component>>,
    pub synchronous: bool,
}

impl Default for RouterConfiguration {
    fn default() -> Self {
        Self {
            failure_external_navigation: todo!("provide default component for external navigation"),
            failure_named_navigation: todo!("provide default component for named navigation"),
            failure_redirection_limit: todo!("provide default component for redirection limit"),
            history: Box::new(MemoryHistory::default()),
            on_update: None,
            synchronous: false,
        }
    }
}
