use std::{cell::RefCell, rc::Rc};

use crate::{Routable, RouterCfg, RouterService};
use dioxus_core::ScopeState;

/// Initialize the app's router service and provide access to `Link` components
pub fn use_router<'a, R: Routable>(cx: &'a ScopeState, cfg: impl FnOnce(&mut RouterCfg)) -> &'a R {
    cx.use_hook(
        |_| {
            let svc: RouterService<R> = RouterService {
                regen_route: cx.schedule_update(),
                pending_routes: RefCell::new(Vec::new()),
            };
            let first_path = R::default();
            cx.provide_context(svc);
            UseRouterInner {
                svc: cx.consume_context::<RouterService<R>>().unwrap(),
                history: vec![first_path],
            }
        },
        |f| {
            let mut pending_routes = f.svc.pending_routes.borrow_mut();

            for route in pending_routes.drain(..) {
                f.history.push(route);
            }

            f.history.last().unwrap()
        },
    )
}

struct UseRouterInner<R: Routable> {
    svc: Rc<RouterService<R>>,
    history: Vec<R>,
}
