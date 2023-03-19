//! Widget utilities for defining, registering and updating widgets

use std::sync::{Arc, RwLock};

use rustc_hash::FxHashMap;
use shipyard::Component;

use crate::{
    node::{FromAnyValue, NodeType},
    node_ref::AttributeMask,
    node_watcher::{AttributeWatcher, NodeWatcher},
    prelude::{NodeImmutable, NodeMut, RealDom},
    NodeId,
};

/// A watcher that handlers registering and updating widgets
#[derive(Default, Clone)]
pub struct WidgetWatcher<V: FromAnyValue + Send + Sync> {
    inner: Arc<RwLock<WidgetWatcherInner<V>>>,
}

impl<V: FromAnyValue + Send + Sync> NodeWatcher<V> for WidgetWatcher<V> {
    fn on_node_added(&mut self, node: NodeMut<V>) {
        let mut inner = self.inner.write().unwrap();
        inner.on_node_added(node);
    }

    fn on_node_removed(&mut self, node: NodeMut<V>) {
        let mut inner = self.inner.write().unwrap();
        inner.on_node_removed(node);
    }
}

impl<V: FromAnyValue + Send + Sync> WidgetWatcher<V> {
    /// Register a widget
    pub fn register_widget<W: WidgetFactory<O, V> + 'static, O: WidgetUpdater<V>>(&mut self) {
        let mut inner = self.inner.write().unwrap();
        inner.builders.insert(
            W::NAME,
            WidgetBuilder {
                create: |mut node| Box::new(W::create(&mut node)),
            },
        );
    }

    /// Attach the widget watcher to the RealDom
    pub fn attach(&self, dom: &mut RealDom<V>) {
        dom.add_node_watcher(self.clone());
        dom.add_attribute_watcher(self.clone());
    }
}

impl<V: FromAnyValue + Send + Sync> AttributeWatcher<V> for WidgetWatcher<V> {
    fn on_attributes_changed(&self, node: NodeMut<V>, attributes: &AttributeMask) {
        let mut inner = self.inner.write().unwrap();
        if let Some(widget) = inner.widgets.get_mut(&node.id()) {
            widget.dyn_widget.attributes_changed(node, attributes);
        }
    }
}

#[derive(Default)]
struct WidgetWatcherInner<V: FromAnyValue + Send + Sync> {
    builders: FxHashMap<&'static str, WidgetBuilder<V>>,
    widgets: FxHashMap<NodeId, BoxedWidget<V>>,
}

impl<V: FromAnyValue + Send + Sync> NodeWatcher<V> for WidgetWatcherInner<V> {
    fn on_node_added(&mut self, node: NodeMut<V>) {
        let node_type = node.node_type();
        if let NodeType::Element(el) = &*node_type {
            if let Some(builder) = self.builders.get(el.tag.as_str()) {
                drop(node_type);
                let id = node.id();
                let widget = (builder.create)(node);
                self.widgets.insert(id, BoxedWidget { dyn_widget: widget });
            }
        }
    }

    fn on_node_removed(&mut self, node: NodeMut<V>) {
        self.widgets.remove(&node.id());
    }
}

#[derive(Component)]
struct BoxedWidget<V: FromAnyValue + Send + Sync> {
    dyn_widget: Box<dyn WidgetUpdater<V>>,
}

struct WidgetBuilder<V: FromAnyValue + Send + Sync> {
    create: fn(NodeMut<V>) -> Box<dyn WidgetUpdater<V>>,
}

/// A controlled element (a.k.a. widget)
pub trait Widget<V: FromAnyValue + Send + Sync = ()>: Send + Sync + 'static {
    /// The tag the widget is registered under.
    const NAME: &'static str;

    /// Create a new widget.
    fn create(root: &mut NodeMut<V>) -> Self;

    /// Called when the attributes of the widget are changed.
    fn attributes_changed(&mut self, _root: NodeMut<V>, _attributes: &AttributeMask);
}

/// A factory for creating widgets
pub trait WidgetFactory<W: WidgetUpdater<V>, V: FromAnyValue + Send + Sync = ()>:
    Send + Sync + 'static
{
    /// The tag the widget is registered under.
    const NAME: &'static str;

    /// Create a new widget.
    fn create(root: &mut NodeMut<V>) -> W;
}

impl<W: Widget<V>, V: FromAnyValue + Send + Sync> WidgetFactory<W, V> for W {
    const NAME: &'static str = W::NAME;

    fn create(root: &mut NodeMut<V>) -> Self {
        W::create(root)
    }
}

/// A trait for updating widgets
pub trait WidgetUpdater<V: FromAnyValue + Send + Sync = ()>: Send + Sync + 'static {
    /// Called when the attributes of the widget are changed.
    fn attributes_changed(&mut self, _root: NodeMut<V>, _attributes: &AttributeMask);
}

impl<W: Widget<V>, V: FromAnyValue + Send + Sync> WidgetUpdater<V> for W {
    fn attributes_changed(&mut self, root: NodeMut<V>, attributes: &AttributeMask) {
        self.attributes_changed(root, attributes);
    }
}
