//! Integration between Dioxus and Blitz
use crate::NodeId;
use crate::events::{
    BlitzKeyboardData, NativeConverter, NativeFocusData, NativeFormData, NativePointerData,
    NativeScrollData, NativeTouchData, NativeWheelData, NodeHandle,
};
use crate::mutation_writer::DioxusState;
use crate::qual_name;
use blitz_dom::{
    Attribute, BaseDocument, DocGuard, DocGuardMut, Document, DocumentConfig, EventDriver,
    EventHandler, Node,
};
use blitz_traits::events::{DomEvent, DomEventData, EventState, UiEvent};
use dioxus_core::{ElementId, Event, VirtualDom};
use dioxus_html::{PlatformEventData, set_event_converter};
use futures_util::task::noop_waker;
use std::cell::RefCell;
use std::future::Future;
use std::mem;
use std::pin::pin;
use std::sync::LazyLock;
use std::task::{Context as TaskContext, Waker};
use std::{any::Any, rc::Rc};

fn wrap_event_data<T: Any>(value: T) -> Rc<dyn Any> {
    Rc::new(PlatformEventData::new(Box::new(value)))
}

/// Get the value of the "dioxus-data-id" attribute parsed aa usize
fn get_dioxus_id(node: &Node) -> Option<ElementId> {
    node.element_data()?
        .attrs
        .iter()
        .find(|attr| *attr.name.local == *"data-dioxus-id")
        .and_then(|attr| attr.value.parse::<usize>().ok())
        .map(ElementId::from_raw)
}

/// Integrates [`BaseDocument`] from  [`blitz-dom`](blitz_dom)  with [`VirtualDom`] from [`dioxus-core`](dioxus_core)
///
/// ### Example
///
/// ```rust
/// use blitz_traits::shell::{Viewport, ColorScheme};
/// use dioxus_native_dom::{DioxusDocument, DocumentConfig};
/// use dioxus::prelude::*;
///
/// // Example Dioxus app
/// fn app() -> Element {
///     rsx! {
///         div { "Hello, world!" }
///     }
/// }
///
/// fn main() {
///    let vdom = VirtualDom::new(app);
///    let mut doc = DioxusDocument::new(vdom, DocumentConfig {
///         viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
///         ..Default::default()
///    });
///    doc.initial_build();
/// }
/// ```
///
/// You can just push events into the [`DioxusDocument`] with [`doc.handle_ui_event(..)`](Self::handle_ui_event)
/// and then flush the changes with [`doc.poll(..)`](Self::poll)
pub struct DioxusDocument {
    pub inner: Rc<RefCell<BaseDocument>>,
    pub vdom: VirtualDom,
    pub vdom_state: DioxusState,

    #[allow(unused)]
    pub(crate) html_element_id: NodeId,
    #[allow(unused)]
    pub(crate) head_element_id: NodeId,
    #[allow(unused)]
    pub(crate) body_element_id: NodeId,
    #[allow(unused)]
    pub(crate) main_element_id: NodeId,
}

impl DioxusDocument {
    /// Create a new [`DioxusDocument`] from a [`VirtualDom`].
    pub fn new(vdom: VirtualDom, mut config: DocumentConfig) -> Self {
        // Only really needs to happen once
        set_event_converter(Box::new(NativeConverter {}));

        config.base_url = Some(
            config
                .base_url
                .unwrap_or_else(|| String::from("dioxus://index.html")),
        );
        let mut doc = BaseDocument::new(config);

        // Create some minimal HTML to render the app into.
        // HTML is equivalent to:
        //
        // <html>
        // <head></head>
        // <body>
        //    <div id="main"></div>
        // </body>
        // </html>
        //
        // TODO: Support arbitrary "index.html" templates

        // Create the html element
        let mut mutr = doc.mutate();
        let html_element_id = mutr.create_element(qual_name("html", None), vec![]);
        mutr.append_children(mutr.doc.root_node().id, &[html_element_id]);

        // Create the body element
        let head_element_id = mutr.create_element(qual_name("head", None), vec![]);
        mutr.append_children(html_element_id, &[head_element_id]);

        // Create the body element
        let body_element_id = mutr.create_element(qual_name("body", None), vec![]);
        mutr.append_children(html_element_id, &[body_element_id]);

        // Create another virtual element to hold the root <div id="main"></div> under the html element
        let main_attr = blitz_dom::Attribute {
            name: qual_name("id", None),
            value: "main".to_string(),
        };
        let main_element_id = mutr.create_element(qual_name("main", None), vec![main_attr]);
        mutr.append_children(body_element_id, &[main_element_id]);

        drop(mutr);

        let vdom_state = DioxusState::create(main_element_id);
        Self {
            vdom,
            vdom_state,
            inner: Rc::new(RefCell::new(doc)),
            html_element_id,
            head_element_id,
            body_element_id,
            main_element_id,
        }
    }

    /// Run an initial build of the Dioxus vdom
    pub fn initial_build(&mut self) {
        // Mutations are buffered and only applied once the vdom has finished running. This means
        // that the document is not borrowed while user code (which may access it) is running.
        self.vdom.rebuild(&mut self.vdom_state.buffered_mutations);
        self.vdom_state
            .apply_buffered_mutations(&mut self.inner.borrow_mut());
        self.flush_queued_mounted_events();
    }

    /// Used to respond to a `CreateHeadElement` event generated by Dioxus. These
    /// events allow Dioxus to create elements in the `<head>` of the document.
    #[doc(hidden)]
    pub fn create_head_element(
        &mut self,
        name: &str,
        attributes: &[(String, String)],
        contents: &Option<String>,
    ) {
        let mut inner = self.inner.borrow_mut();
        let mut mutr = inner.mutate();

        let attributes = attributes
            .iter()
            .map(|(name, value)| Attribute {
                name: qual_name(name, None),
                value: value.clone(),
            })
            .collect();

        let new_elem_id = mutr.create_element(qual_name(name, None), attributes);
        mutr.append_children(self.head_element_id, &[new_elem_id]);
        if let Some(contents) = contents {
            let text_node_id = mutr.create_text_node(contents);
            mutr.append_children(new_elem_id, &[text_node_id]);
        }
    }

    pub(crate) fn flush_queued_mounted_events(&mut self) {
        let mut queued_mounted_events = mem::take(&mut self.vdom_state.queued_mounted_events);
        for element_id in queued_mounted_events.drain(..) {
            let node_id = self.vdom_state.element_to_node_id(element_id);

            if self.inner.borrow().get_node(node_id).is_some() {
                let event = Event::new(
                    Rc::new(PlatformEventData::new(Box::new(NodeHandle {
                        doc: Rc::clone(&self.inner),
                        node_id,
                    }))) as Rc<dyn Any>,
                    false,
                );
                self.vdom
                    .runtime()
                    .handle_event("mounted", event, element_id);
            }
        }

        self.vdom_state.queued_mounted_events = queued_mounted_events;
    }
}

// Implement DocumentLike and required traits for DioxusDocument
impl Document for DioxusDocument {
    fn id(&self) -> usize {
        self.inner.borrow().id()
    }

    fn inner(&self) -> DocGuard<'_> {
        DocGuard::RefCell(self.inner.borrow())
    }

    fn inner_mut(&mut self) -> DocGuardMut<'_> {
        DocGuardMut::RefCell(self.inner.borrow_mut())
    }

    fn poll(&mut self, cx: Option<TaskContext>) -> bool {
        static NOOP_WAKER: LazyLock<Waker> = LazyLock::new(noop_waker);
        let waker = cx
            .as_ref()
            .map(|cx| cx.waker().clone())
            .unwrap_or_else(|| NOOP_WAKER.clone());

        // Poll any sub-documents, which may have pending async operations of
        // their own (e.g. JavaScript timers)
        let subdoc_changes = self.inner.borrow_mut().poll_subdocuments(Some(&waker));

        {
            let fut = self.vdom.wait_for_work();
            let mut pinned_fut = pin!(fut);

            let mut cx = TaskContext::from_waker(&waker);
            match pinned_fut.as_mut().poll(&mut cx) {
                std::task::Poll::Ready(_) => {}
                std::task::Poll::Pending => return subdoc_changes,
            }
        }

        // Mutations are buffered and only applied once the vdom has finished running. This means
        // that the document is not borrowed while user code (which may access it) is running.
        self.vdom
            .render_immediate(&mut self.vdom_state.buffered_mutations);
        self.vdom_state
            .apply_buffered_mutations(&mut self.inner.borrow_mut());
        self.flush_queued_mounted_events();

        true
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        let handler = DioxusEventHandler {
            vdom: &mut self.vdom,
            vdom_state: &mut self.vdom_state,
        };
        let mut driver = EventDriver::new(&mut self.inner, handler);
        driver.handle_ui_event(event);
    }
}

pub struct DioxusEventHandler<'v> {
    vdom: &'v mut VirtualDom,
    vdom_state: &'v mut DioxusState,
}

impl EventHandler for DioxusEventHandler<'_> {
    fn handle_event(
        &mut self,
        chain: &[NodeId],
        event: &mut DomEvent,
        doc: &mut dyn Document,
        event_state: &mut EventState,
    ) {
        // As an optimisation we maintain a count of the total number event handlers of a given type
        // If this count is zero then we can skip handling that kind of event entirely.
        let event_kind_idx = event.data.discriminant() as usize;
        let event_kind_count = self.vdom_state.event_handler_counts[event_kind_idx];
        if event_kind_count == 0 {
            return;
        }

        let event_data = match &event.data {
            DomEventData::PointerMove(mevent)
            | DomEventData::PointerDown(mevent)
            | DomEventData::PointerUp(mevent)
            | DomEventData::PointerCancel(mevent)
            | DomEventData::PointerLeave(mevent)
            | DomEventData::PointerEnter(mevent)
            | DomEventData::PointerOver(mevent)
            | DomEventData::PointerOut(mevent)
            | DomEventData::MouseMove(mevent)
            | DomEventData::MouseDown(mevent)
            | DomEventData::MouseUp(mevent)
            | DomEventData::MouseLeave(mevent)
            | DomEventData::MouseEnter(mevent)
            | DomEventData::MouseOver(mevent)
            | DomEventData::MouseOut(mevent)
            | DomEventData::Click(mevent)
            | DomEventData::ContextMenu(mevent)
            | DomEventData::DoubleClick(mevent) => {
                Some(wrap_event_data(NativePointerData(mevent.clone())))
            }

            DomEventData::TouchStart(tevent)
            | DomEventData::TouchMove(tevent)
            | DomEventData::TouchEnd(tevent)
            | DomEventData::TouchCancel(tevent) => {
                Some(wrap_event_data(NativeTouchData(tevent.clone())))
            }

            DomEventData::Scroll(sevent) => Some(wrap_event_data(NativeScrollData(sevent.clone()))),
            DomEventData::Wheel(wevent) => Some(wrap_event_data(NativeWheelData(wevent.clone()))),

            DomEventData::Focus(_) => Some(wrap_event_data(NativeFocusData)),
            DomEventData::Blur(_) => Some(wrap_event_data(NativeFocusData)),
            DomEventData::FocusIn(_) => Some(wrap_event_data(NativeFocusData)),
            DomEventData::FocusOut(_) => Some(wrap_event_data(NativeFocusData)),

            DomEventData::KeyDown(kevent)
            | DomEventData::KeyUp(kevent)
            | DomEventData::KeyPress(kevent) => {
                Some(wrap_event_data(BlitzKeyboardData(kevent.clone())))
            }

            DomEventData::Input(data) => Some(wrap_event_data(NativeFormData {
                value: data.value.clone(),
                values: vec![],
            })),

            // TODO: Implement IME handling
            DomEventData::Ime(_) => None,

            // AppleStandardKeybinding events are not exposed to script
            DomEventData::AppleStandardKeybinding(_) => None,
        };

        let Some(event_data) = event_data else {
            return;
        };

        // Get dioxus vdom id for node
        let dioxus_id = chain
            .iter()
            .find_map(|node_id| doc.inner().get_node(*node_id).and_then(get_dioxus_id));
        let Some(id) = dioxus_id else {
            return;
        };

        // Handle event in vdom
        let dx_event = Event::new(event_data.clone(), event.bubbles);
        self.vdom
            .runtime()
            .handle_event(event.name(), dx_event.clone(), id);

        // Update event state
        if !dx_event.default_action_enabled() {
            event_state.prevent_default();
        }
        if !dx_event.propagates() {
            event_state.stop_propagation();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blitz_dom::DocumentConfig;
    use dioxus::prelude::*;
    use dioxus_core::ScopeId;
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    #[test]
    // Regression test for a panic. The keyed `div`s are re-ordered as the (unordered) `HashMap` grows,
    // which previously caused a crash when moving keyed nodes within their parent.
    fn keyed_nodes_do_not_crash() {
        type SharedData = Rc<RefCell<HashMap<usize, usize>>>;
        let data: SharedData = Rc::new(RefCell::new(HashMap::new()));

        fn app(data: SharedData) -> Element {
            let entries: Vec<usize> = data.borrow().keys().copied().collect();
            rsx!(
                for id in entries {
                    div {
                        key: "item_{id}",
                        "{id}"
                    }
                }
            )
        }

        let vdom = VirtualDom::new_with_props(app, Rc::clone(&data));
        let mut doc = DioxusDocument::new(vdom, DocumentConfig::default());
        doc.initial_build();

        // Mirror `examples/crash.rs`: incrementally insert 100 items, flushing
        // the resulting mutations after each insert.
        for i in 0..100 {
            data.borrow_mut().insert(i, i);
            doc.vdom.mark_dirty(ScopeId::APP);
            doc.poll(None);
        }

        // The `<main>` element should end up with exactly one keyed `div` per
        // inserted item, and applying the mutations must not have panicked.
        let inner = doc.inner.borrow();
        let main = inner.get_node(doc.main_element_id).unwrap();
        assert_eq!(main.children.len(), 100);
    }

    fn node_count(doc: &DioxusDocument) -> usize {
        doc.inner.borrow().tree().len()
    }

    fn rerender(doc: &mut DioxusDocument) {
        doc.vdom.mark_dirty(ScopeId::APP);
        poll_until_idle(doc);
    }

    fn poll_until_idle(doc: &mut DioxusDocument) {
        for _ in 0..8 {
            if !doc.poll(None) {
                return;
            }
        }
        panic!("document did not become idle");
    }

    fn main_text(doc: &DioxusDocument) -> String {
        let inner = doc.inner.borrow();
        let mut text = String::new();
        let mut stack = vec![doc.main_element_id];
        while let Some(id) = stack.pop() {
            let node = inner.get_node(id).unwrap();
            if let Some(data) = node.text_data() {
                text.push_str(&data.content);
            }
            stack.extend(node.children.iter().rev().copied());
        }
        text
    }

    #[test]
    fn removed_nodes_are_dropped() {
        fn app() -> Element {
            let item_count = consume_context::<Rc<Cell<usize>>>();
            rsx!(
                for id in 0..item_count.get() {
                    div { key: "{id}", span { "{id}" } }
                }
            )
        }

        let item_count = Rc::new(Cell::new(0usize));
        let vdom = VirtualDom::new(app).with_root_context(Rc::clone(&item_count));
        let mut doc = DioxusDocument::new(vdom, DocumentConfig::default());
        doc.initial_build();

        let cycle = |doc: &mut DioxusDocument| {
            item_count.set(50);
            rerender(doc);
            let main_id = doc.main_element_id;
            assert_eq!(
                doc.inner.borrow().get_node(main_id).unwrap().children.len(),
                50
            );
            item_count.set(0);
            rerender(doc);
            node_count(doc)
        };

        // The first cycle creates the (retained) template prototype nodes.
        let baseline = cycle(&mut doc);
        for _ in 0..5 {
            assert_eq!(cycle(&mut doc), baseline);
        }
    }

    #[test]
    fn suspended_nodes_are_dropped_and_recreated() {
        #[derive(Default)]
        struct State {
            suspended: Cell<bool>,
            generation: Cell<usize>,
            waker: Cell<Option<std::task::Waker>>,
        }

        // `generation` only exists to defeat memoization so that the component re-renders
        #[component]
        fn Child(generation: usize) -> Element {
            let state = consume_context::<Rc<State>>();
            let mut resolved = use_signal(|| false);
            if state.suspended.get() {
                resolved.set(false);
                let task = spawn(async move {
                    std::future::poll_fn(|cx| {
                        if state.suspended.get() {
                            state.waker.set(Some(cx.waker().clone()));
                            std::task::Poll::Pending
                        } else {
                            std::task::Poll::Ready(())
                        }
                    })
                    .await;
                    resolved.set(true);
                });
                suspend(task)?;
            }
            rsx!( div { span { "content" } } )
        }

        fn app() -> Element {
            let state = consume_context::<Rc<State>>();
            rsx!(
                SuspenseBoundary {
                    fallback: |_| rsx!( p { "loading" } ),
                    Child { generation: state.generation.get() }
                }
            )
        }

        let state = Rc::new(State::default());
        let vdom = VirtualDom::new(app).with_root_context(Rc::clone(&state));
        let mut doc = DioxusDocument::new(vdom, DocumentConfig::default());
        doc.initial_build();
        assert_eq!(main_text(&doc), "content");

        let cycle = |doc: &mut DioxusDocument| {
            state.suspended.set(true);
            state.generation.set(state.generation.get() + 1);
            rerender(doc);
            assert_eq!(main_text(doc), "loading");
            state.suspended.set(false);
            state.waker.take().unwrap().wake();
            poll_until_idle(doc);
            assert_eq!(main_text(doc), "content");
            node_count(doc)
        };

        let baseline = cycle(&mut doc);
        for _ in 0..5 {
            assert_eq!(cycle(&mut doc), baseline);
        }
    }

    #[test]
    fn todo_like_stress() {
        use std::cell::RefCell;
        #[derive(Clone, PartialEq)]
        struct Item {
            id: u32,
            checked: bool,
            editing: bool,
        }
        #[derive(Default)]
        struct State {
            items: RefCell<Vec<Item>>,
            filter: Cell<u8>,
        }

        #[component]
        fn Entry(item: Item) -> Element {
            rsx! {
                li { class: if item.checked { "completed" },
                    div { class: "view",
                        input { r#type: "checkbox", id: "cbg-{item.id}" }
                        label { "todo {item.id}" }
                        button { class: "destroy" }
                    }
                    if item.editing {
                        input { class: "edit", value: "todo {item.id}" }
                    }
                }
            }
        }

        fn app() -> Element {
            let state = consume_context::<Rc<State>>();
            let items = state.items.borrow().clone();
            let filter = state.filter.get();
            let active = items.iter().filter(|i| !i.checked).count();
            let visible: Vec<Item> = items
                .iter()
                .filter(|i| match filter {
                    1 => !i.checked,
                    2 => i.checked,
                    _ => true,
                })
                .cloned()
                .collect();
            rsx! {
                section {
                    if !items.is_empty() {
                        input { id: "toggle-all", r#type: "checkbox" }
                        label { r#for: "toggle-all" }
                    }
                    ul {
                        for item in visible {
                            Entry { key: "{item.id}", item }
                        }
                    }
                    if !items.is_empty() {
                        footer {
                            span { strong { "{active} " } span { match active { 1 => "item", _ => "items" } " left" } }
                            if items.iter().any(|i| i.checked) {
                                button { "Clear completed" }
                            }
                        }
                    }
                }
            }
        }

        let state = Rc::new(State::default());
        let vdom = VirtualDom::new(app).with_root_context(Rc::clone(&state));
        let mut doc = DioxusDocument::new(vdom, DocumentConfig::default());
        doc.initial_build();

        let mut seed = 0x2545F4914F6CDD1Du64;
        let mut rng = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut next_id = 0;
        let mut counts = Vec::new();
        for step in 1..=3000 {
            let r = rng();
            {
                let mut items = state.items.borrow_mut();
                let len = items.len();
                match r % 8 {
                    0 | 1 => {
                        items.push(Item {
                            id: next_id,
                            checked: false,
                            editing: false,
                        });
                        next_id += 1;
                    }
                    2 if len > 0 => {
                        items.remove((r >> 8) as usize % len);
                    }
                    3 if len > 0 => {
                        let i = (r >> 8) as usize % len;
                        items[i].checked ^= true;
                    }
                    4 if len > 0 => {
                        let i = (r >> 8) as usize % len;
                        items[i].editing ^= true;
                    }
                    5 => state.filter.set(((r >> 8) % 3) as u8),
                    6 => items.retain(|i| !i.checked),
                    7 if len > 12 => items.clear(),
                    _ => {}
                }
            }
            rerender(&mut doc);
            if step % 1000 == 0 {
                state.items.borrow_mut().clear();
                rerender(&mut doc);
                counts.push(node_count(&doc));
            }
        }
        // The first round creates the (retained) template prototype nodes.
        assert_eq!(counts[1], counts[2]);
    }

    #[test]
    // Regression test for a panic. Tasks are polled while the vdom renders, at which point
    // the document must not be borrowed.
    fn task_polled_during_render_can_access_document() {
        #[derive(Default)]
        struct State {
            focus: Cell<bool>,
            mounted: RefCell<Option<Rc<MountedData>>>,
        }

        fn app() -> Element {
            let state = consume_context::<Rc<State>>();
            if state.focus.get() {
                let mounted = state.mounted.borrow().clone().unwrap();
                spawn(async move {
                    mounted.set_focus(true).await.unwrap();
                    mounted.get_client_rect().await.unwrap();
                });
            }
            let focus = state.focus.get();
            rsx!(input {
                "data-focus": "{focus}",
                onmounted: move |e| *state.mounted.borrow_mut() = Some(e.data()),
            })
        }

        let state = Rc::new(State::default());
        let vdom = VirtualDom::new(app).with_root_context(Rc::clone(&state));
        let mut doc = DioxusDocument::new(vdom, DocumentConfig::default());
        doc.initial_build();
        state.focus.set(true);
        rerender(&mut doc);
        let inner = doc.inner.borrow();
        let focussed = inner.get_node(inner.get_focussed_node_id().unwrap());
        assert_eq!(
            focussed.unwrap().attr(crate::LocalName::from("data-focus")),
            Some("true")
        );
    }

    #[test]
    fn mounted_data_of_removed_element_reports_error() {
        #[derive(Default)]
        struct State {
            show: Cell<bool>,
            mounted: RefCell<Option<Rc<MountedData>>>,
        }

        fn app() -> Element {
            let state = consume_context::<Rc<State>>();
            rsx!(if state.show.get() {
                div {
                    onmounted: move |e| *state.mounted.borrow_mut() = Some(e.data()),
                }
            })
        }

        let state = Rc::new(State::default());
        state.show.set(true);
        let vdom = VirtualDom::new(app).with_root_context(Rc::clone(&state));
        let mut doc = DioxusDocument::new(vdom, DocumentConfig::default());
        doc.initial_build();
        let mounted = state.mounted.take().unwrap();
        let focus = |focus: bool| {
            let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
            let fut = std::pin::pin!(mounted.set_focus(focus));
            matches!(fut.poll(&mut cx), std::task::Poll::Ready(Ok(())))
        };
        assert!(focus(true));
        let focussed = |doc: &DioxusDocument| doc.inner.borrow().get_focussed_node_id();
        let mounted_id = focussed(&doc).unwrap();
        state.show.set(false);
        rerender(&mut doc);
        assert!(doc.inner.borrow().get_node(mounted_id).is_none());
        assert_ne!(focussed(&doc), Some(mounted_id));

        // Focussing or blurring the removed element is a no-op.
        let focussed_after_removal = focussed(&doc);
        assert!(focus(true));
        assert!(focus(false));
        assert_eq!(focussed(&doc), focussed_after_removal);

        fn is_err<T>(fut: impl Future<Output = Result<T, MountedError>>) -> bool {
            let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
            let fut = std::pin::pin!(fut);
            matches!(fut.poll(&mut cx), std::task::Poll::Ready(Err(_)))
        }
        assert!(is_err(mounted.get_scroll_offset()));
        assert!(is_err(mounted.get_scroll_size()));
        assert!(is_err(mounted.get_client_rect()));
        assert!(is_err(mounted.scroll_to(ScrollBehavior::Instant)));
        assert!(is_err(
            mounted.scroll(Default::default(), ScrollBehavior::Instant)
        ));
    }
}
