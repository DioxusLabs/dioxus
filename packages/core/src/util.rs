use std::{
    cell::{Cell, RefCell, RefMut},
    rc::Rc,
};

use futures_util::StreamExt;

use crate::innerlude::*;

#[derive(PartialEq, Debug, Clone, Default)]
pub struct EventQueue {
    pub queue: Rc<RefCell<Vec<HeightMarker>>>,
}

impl EventQueue {
    pub fn new_channel(&self, height: u32, idx: ScopeIdx) -> Rc<dyn Fn()> {
        let inner = self.clone();
        let marker = HeightMarker { height, idx };
        Rc::new(move || {
            log::debug!("channel updated {:#?}", marker);
            inner.queue.as_ref().borrow_mut().push(marker)
        })
    }

    pub fn sort_unstable(&self) {
        self.queue.borrow_mut().sort_unstable()
    }

    pub fn borrow_mut(&self) -> RefMut<Vec<HeightMarker>> {
        self.queue.borrow_mut()
    }
}

/// A helper type that lets scopes be ordered by their height
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeightMarker {
    pub idx: ScopeIdx,
    pub height: u32,
}

impl Ord for HeightMarker {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height)
    }
}

impl PartialOrd for HeightMarker {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The `RealDomNode` is an ID handle that corresponds to a foreign DOM node.
///
/// "u64" was chosen for two reasons
/// - 0 cost hashing
/// - use with slotmap and other versioned slot arenas
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RealDomNode(pub u64);
impl RealDomNode {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    pub const fn empty() -> Self {
        Self(u64::MIN)
    }
    pub const fn empty_cell() -> Cell<Self> {
        Cell::new(Self::empty())
    }
}

pub struct DebugDom {
    counter: u64,
    logging: bool,
}
impl DebugDom {
    pub fn new() -> Self {
        Self {
            counter: 0,
            logging: false,
        }
    }
    pub fn with_logging_enabled() -> Self {
        Self {
            counter: 0,
            logging: true,
        }
    }
}
impl<'a> RealDom<'a> for DebugDom {
    fn push(&mut self, _root: RealDomNode) {}
    fn pop(&mut self) {}

    fn append_children(&mut self, _many: u32) {}

    fn replace_with(&mut self, _many: u32) {}

    fn remove(&mut self) {}

    fn remove_all_children(&mut self) {}

    fn create_text_node(&mut self, _text: &str) -> RealDomNode {
        self.counter += 1;
        RealDomNode::new(self.counter)
    }

    fn create_element(&mut self, _tag: &str, _ns: Option<&'a str>) -> RealDomNode {
        self.counter += 1;
        RealDomNode::new(self.counter)
    }

    fn create_placeholder(&mut self) -> RealDomNode {
        self.counter += 1;
        RealDomNode::new(self.counter)
    }

    fn new_event_listener(
        &mut self,
        _event: &str,
        _scope: ScopeIdx,
        _element_id: usize,
        _realnode: RealDomNode,
    ) {
    }

    fn remove_event_listener(&mut self, _event: &str) {}

    fn set_text(&mut self, _text: &str) {}

    fn set_attribute(&mut self, _name: &str, _value: &str, _namespace: Option<&str>) {}

    fn remove_attribute(&mut self, _name: &str) {}

    fn raw_node_as_any_mut(&self) -> &mut dyn std::any::Any {
        todo!()
    }
}

async fn launch_demo(app: FC<()>) {
    let mut dom = VirtualDom::new(app);
    let mut real_dom = DebugDom::new();
    dom.rebuild(&mut real_dom).unwrap();

    while let Some(evt) = dom.tasks.next().await {
        //
        log::debug!("Event triggered! {:#?}", evt);
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate as dioxus;
//     use std::{pin::Pin, time::Duration};

//     use crate::builder::DioxusElement;
//     use dioxus::prelude::*;
//     use futures::Future;

//     #[async_std::test]
//     async fn async_tick() {
//         static App: FC<()> = |cx| {
//             // let mut count = use_state(cx, || 0);
//             let mut fut = cx.use_hook(
//                 move || {
//                     Box::pin(async {
//                         //
//                         let mut tick: i32 = 0;
//                         loop {
//                             async_std::task::sleep(Duration::from_millis(250)).await;
//                             log::debug!("ticking forward... {}", tick);
//                             tick += 1;
//                             // match surf::get(ENDPOINT).recv_json::<DogApi>().await {
//                             //     Ok(_) => (),
//                             //     Err(_) => (),
//                             // }
//                         }
//                     }) as Pin<Box<dyn Future<Output = ()> + 'static>>
//                 },
//                 |h| h,
//                 |_| {},
//             );

//             cx.submit_task(fut);

//             cx.render(LazyNodes::new(move |f| {
//                 f.text(format_args!("it's sorta working"))
//             }))
//         };

//         std::env::set_var("RUST_LOG", "debug");
//         env_logger::init();
//         launch_demo(App).await;
//     }
// }
