#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

mod element;

use std::{
    any::Any,
    ops::Deref,
    rc::Rc,
    sync::{Arc, RwLock},
};

use dioxus_core::{Component, ElementId, VirtualDom};
use dioxus_native_core::dioxus::{DioxusState, NodeImmutableDioxusExt};
use dioxus_native_core::prelude::*;

use element::{create_mounted_events, find_mount_events};
pub use plasmo::{query::Query, Config, RenderingMode, Size, TuiContext};
use plasmo::{render, Driver};

pub fn launch(app: Component<()>) {
    launch_cfg(app, Config::default())
}

pub fn launch_cfg(app: Component<()>, cfg: Config) {
    launch_cfg_with_props(app, (), cfg);
}

pub fn launch_cfg_with_props<Props: 'static>(app: Component<Props>, props: Props, cfg: Config) {
    render(cfg, |rdom, taffy, event_tx| {
        let dioxus_state = {
            let mut rdom = rdom.write().unwrap();
            DioxusState::create(&mut rdom)
        };
        let dioxus_state = Rc::new(RwLock::new(dioxus_state));
        let mut vdom = VirtualDom::new_with_props(app, props)
            .with_root_context(TuiContext::new(event_tx))
            .with_root_context(Query::new(rdom.clone(), taffy.clone()))
            .with_root_context(DioxusElementToNodeId {
                mapping: dioxus_state.clone(),
            });
        let muts = vdom.rebuild();

        let mut queued_events = Vec::new();

        {
            let mut rdom = rdom.write().unwrap();
            let mut dioxus_state = dioxus_state.write().unwrap();

            // Find any mount events
            let mounted = find_mount_events(&muts);

            dioxus_state.apply_mutations(&mut rdom, muts);

            // Send the mount events
            create_mounted_events(
                &vdom,
                &mut queued_events,
                mounted
                    .iter()
                    .map(|id| (*dbg!(id), dioxus_state.element_to_node_id(*id))),
            );
        }

        DioxusRenderer {
            vdom,
            dioxus_state,
            queued_events,
            #[cfg(all(feature = "hot-reload", debug_assertions))]
            hot_reload_rx: {
                let (hot_reload_tx, hot_reload_rx) =
                    tokio::sync::mpsc::unbounded_channel::<dioxus_hot_reload::HotReloadMsg>();
                dioxus_hot_reload::connect(move |msg| {
                    let _ = hot_reload_tx.send(msg);
                });
                hot_reload_rx
            },
        }
    })
    .unwrap();
}

struct DioxusRenderer {
    vdom: VirtualDom,
    dioxus_state: Rc<RwLock<DioxusState>>,
    // Events that are queued up to be sent to the vdom next time the vdom is polled
    queued_events: Vec<(ElementId, &'static str, Rc<dyn Any>, bool)>,
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    hot_reload_rx: tokio::sync::mpsc::UnboundedReceiver<dioxus_hot_reload::HotReloadMsg>,
}

impl Driver for DioxusRenderer {
    fn update(&mut self, rdom: &Arc<RwLock<RealDom>>) {
        let muts = self.vdom.render_immediate();
        {
            let mut rdom = rdom.write().unwrap();

            {
                // Find any mount events
                let mounted = find_mount_events(&muts);

                let mut dioxus_state = self.dioxus_state.write().unwrap();
                dioxus_state.apply_mutations(&mut rdom, muts);

                // Send the mount events
                create_mounted_events(
                    &self.vdom,
                    &mut self.queued_events,
                    mounted
                        .iter()
                        .map(|id| (*id, dioxus_state.element_to_node_id(*id))),
                );
            }
        }
    }

    fn handle_event(
        &mut self,
        rdom: &Arc<RwLock<RealDom>>,
        id: NodeId,
        event: &str,
        value: Rc<plasmo::EventData>,
        bubbles: bool,
    ) {
        let id = { rdom.read().unwrap().get(id).unwrap().mounted_id() };
        if let Some(id) = id {
            let inner_value = value.deref().clone();
            self.vdom
                .handle_event(event, inner_value.into_any(), id, bubbles);
        }
    }

    fn poll_async(&mut self) -> std::pin::Pin<Box<dyn futures::Future<Output = ()> + '_>> {
        // Add any queued events
        for (id, event, value, bubbles) in self.queued_events.drain(..) {
            self.vdom.handle_event(event, value, id, bubbles);
        }

        #[cfg(all(feature = "hot-reload", debug_assertions))]
        return Box::pin(async {
            let hot_reload_wait = self.hot_reload_rx.recv();
            let mut hot_reload_msg = None;
            let wait_for_work = self.vdom.wait_for_work();
            tokio::select! {
                Some(msg) = hot_reload_wait => {
                    #[cfg(all(feature = "hot-reload", debug_assertions))]
                    {
                        hot_reload_msg = Some(msg);
                    }
                    #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
                    let () = msg;
                }
                _ = wait_for_work => {}
            }
            // if we have a new template, replace the old one
            if let Some(msg) = hot_reload_msg {
                match msg {
                    dioxus_hot_reload::HotReloadMsg::UpdateTemplate(template) => {
                        self.vdom.replace_template(template);
                    }
                    dioxus_hot_reload::HotReloadMsg::Shutdown => {
                        std::process::exit(0);
                    }
                }
            }
        });

        #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
        Box::pin(self.vdom.wait_for_work())
    }
}

#[derive(Clone)]
pub struct DioxusElementToNodeId {
    mapping: Rc<RwLock<DioxusState>>,
}

impl DioxusElementToNodeId {
    pub fn get_node_id(&self, element_id: ElementId) -> Option<NodeId> {
        self.mapping
            .read()
            .unwrap()
            .try_element_to_node_id(element_id)
    }
}
