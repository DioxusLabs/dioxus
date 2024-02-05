use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard};

use dioxus_native_core::prelude::*;
use shipyard::Unique;
use taffy::{
    geometry::Point,
    prelude::{Layout, Size},
    Taffy,
};

use crate::{get_abs_layout, layout_to_screen_space};

/// Allows querying the layout of nodes after rendering. It will only provide a correct value after a node is rendered.
/// Provided as a root context for all tui applictions.
/// # Example
/// ```rust, ignore
/// use dioxus::prelude::*;
/// use dioxus_tui::query::Query;
/// use dioxus_tui::Size;
///
/// fn main() {
///     dioxus_tui::launch(app);
/// }
///
/// fn app() -> Element {
///     let hue = use_signal(|| 0.0);
///     let brightness = use_signal(|| 0.0);
///     let tui_query: Query = cx.consume_context().unwrap();
///     rsx! {
///         div{
///             width: "100%",
///             background_color: "hsl({hue}, 70%, {brightness}%)",
///             onmousemove: move |evt| {
///                 let node = tui_query.get(cx.root_node().mounted_id());
///                 let Size{width, height} = node.size().unwrap();
///                 hue.set((evt.data.offset_x as f32/width as f32)*255.0);
///                 brightness.set((evt.data.offset_y as f32/height as f32)*100.0);
///             },
///             "hsl({hue}, 70%, {brightness}%)",
///         }
///     })
/// }
/// ```
#[derive(Clone, Unique)]
pub struct Query {
    pub(crate) rdom: Arc<RwLock<RealDom>>,
    pub(crate) stretch: Arc<Mutex<Taffy>>,
}

impl Query {
    pub fn new(rdom: Arc<RwLock<RealDom>>, stretch: Arc<Mutex<Taffy>>) -> Self {
        Self { rdom, stretch }
    }

    pub fn get(&self, id: NodeId) -> ElementRef {
        let rdom = self.rdom.read();
        let stretch = self.stretch.lock();
        ElementRef::new(
            rdom.expect("rdom lock poisoned"),
            stretch.expect("taffy lock poisoned"),
            id,
        )
    }
}

pub struct ElementRef<'a> {
    inner: RwLockReadGuard<'a, RealDom>,
    stretch: MutexGuard<'a, Taffy>,
    id: NodeId,
}

impl<'a> ElementRef<'a> {
    pub(crate) fn new(
        inner: RwLockReadGuard<'a, RealDom>,
        stretch: MutexGuard<'a, Taffy>,
        id: NodeId,
    ) -> Self {
        Self { inner, stretch, id }
    }

    pub fn size(&self) -> Option<Size<u32>> {
        self.layout().map(|l| l.size.map(|v| v.round() as u32))
    }

    pub fn pos(&self) -> Option<Point<u32>> {
        self.layout().map(|l| Point {
            x: l.location.x.round() as u32,
            y: l.location.y.round() as u32,
        })
    }

    pub fn layout(&self) -> Option<Layout> {
        get_layout(self.inner.get(self.id).unwrap(), &self.stretch)
    }
}

pub(crate) fn get_layout(node: NodeRef, stretch: &Taffy) -> Option<Layout> {
    let layout = get_abs_layout(node, stretch);
    let pos = layout.location;

    Some(Layout {
        order: layout.order,
        size: layout.size.map(layout_to_screen_space),
        location: Point {
            x: layout_to_screen_space(pos.x).round(),
            y: layout_to_screen_space(pos.y).round(),
        },
    })
}
