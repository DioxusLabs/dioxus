//! A custom element is a controlled element that renders to a shadow DOM.
//! Each custom element is registered with a element name

use std::sync::{Arc, RwLock};

use rustc_hash::FxHashMap;
use shipyard::Component;

use crate::{
    node::{FromAnyValue, NodeType},
    node_ref::AttributeMask,
    prelude::{NodeImmutable, NodeMut},
    tree::TreeMut,
    NodeId,
};

pub(crate) struct CustomElementRegistry<V: FromAnyValue + Send + Sync> {
    builders: FxHashMap<(&'static str, Option<&'static str>), CustomElementBuilder<V>>,
}

impl<V: FromAnyValue + Send + Sync> Default for CustomElementRegistry<V> {
    fn default() -> Self {
        Self {
            builders: FxHashMap::default(),
        }
    }
}

impl<V: FromAnyValue + Send + Sync> CustomElementRegistry<V> {
    pub fn register<F, U>(&mut self)
    where
        F: CustomElementFactory<U, V>,
        U: CustomElementUpdater<V>,
    {
        self.builders.insert(
            (F::NAME, F::NAMESPACE),
            CustomElementBuilder {
                create: |node| Box::new(F::create(node)),
            },
        );
    }

    pub fn add_shadow_dom(&self, mut node: NodeMut<V>) {
        let element_tag = if let NodeType::Element(el) = &*node.node_type() {
            Some((el.tag.clone(), el.namespace.clone()))
        } else {
            None
        };
        if let Some((tag, ns)) = element_tag {
            if let Some(builder) = self.builders.get(&(tag.as_str(), ns.as_deref())) {
                let boxed_widget = { (builder.create)(node.reborrow()) };

                let shadow_roots = boxed_widget.roots();

                let light_id = node.id();
                node.real_dom_mut().tree_mut().create_subtree(
                    light_id,
                    shadow_roots,
                    boxed_widget.slot(),
                );

                let boxed_widget = CustomElementManager {
                    inner: Arc::new(RwLock::new(boxed_widget)),
                };

                node.insert(boxed_widget);
            }
        }
    }
}

struct CustomElementBuilder<V: FromAnyValue + Send + Sync> {
    create: fn(NodeMut<V>) -> Box<dyn CustomElementUpdater<V>>,
}

/// A controlled element that renders to a shadow DOM
pub trait CustomElement<V: FromAnyValue + Send + Sync = ()>: Send + Sync + 'static {
    /// The tag the widget is registered under.
    const NAME: &'static str;

    /// The namespace the widget is registered under.
    const NAMESPACE: Option<&'static str> = None;

    /// Create a new widget *without mounting* it.
    fn create(node: NodeMut<V>) -> Self;

    /// The root node of the widget. This must be static once the element is created.
    fn roots(&self) -> Vec<NodeId>;

    /// The slot to render children of the element into. This must be static once the element is created.
    fn slot(&self) -> Option<NodeId> {
        None
    }

    /// Called when the attributes of the widget are changed.
    fn attributes_changed(&mut self, light_node: NodeMut<V>, attributes: &AttributeMask);
}

/// A factory for creating widgets
pub trait CustomElementFactory<W: CustomElementUpdater<V>, V: FromAnyValue + Send + Sync = ()>:
    Send + Sync + 'static
{
    /// The tag the widget is registered under.
    const NAME: &'static str;

    /// The namespace the widget is registered under.
    const NAMESPACE: Option<&'static str> = None;

    /// Create a new widget.
    fn create(dom: NodeMut<V>) -> W;
}

impl<W: CustomElement<V>, V: FromAnyValue + Send + Sync> CustomElementFactory<W, V> for W {
    const NAME: &'static str = W::NAME;

    const NAMESPACE: Option<&'static str> = W::NAMESPACE;

    fn create(node: NodeMut<V>) -> Self {
        Self::create(node)
    }
}

/// A trait for updating widgets
pub trait CustomElementUpdater<V: FromAnyValue + Send + Sync = ()>: Send + Sync + 'static {
    /// Called when the attributes of the widget are changed.
    fn attributes_changed(&mut self, light_root: NodeMut<V>, attributes: &AttributeMask);

    /// The root node of the widget.
    fn roots(&self) -> Vec<NodeId>;

    /// The slot to render children of the element into.
    fn slot(&self) -> Option<NodeId>;
}

impl<W: CustomElement<V>, V: FromAnyValue + Send + Sync> CustomElementUpdater<V> for W {
    fn attributes_changed(&mut self, light_root: NodeMut<V>, attributes: &AttributeMask) {
        self.attributes_changed(light_root, attributes);
    }

    fn roots(&self) -> Vec<NodeId> {
        self.roots()
    }

    fn slot(&self) -> Option<NodeId> {
        self.slot()
    }
}

/// A concrete structure for managing a any widget.
#[derive(Component, Clone)]
pub struct CustomElementManager<V: FromAnyValue = ()> {
    inner: Arc<RwLock<Box<dyn CustomElementUpdater<V>>>>,
}

impl<V: FromAnyValue + Send + Sync> CustomElementManager<V> {
    /// The root node of the widget's shadow DOM.
    pub fn roots(&self) -> Vec<NodeId> {
        self.inner.read().unwrap().roots()
    }

    /// The slot to render children of the element into.
    pub fn slot(&self) -> Option<NodeId> {
        self.inner.read().unwrap().slot()
    }

    /// Update the custom element based on attributes changed.
    pub fn on_attributes_changed(&self, light_root: NodeMut<V>, attributes: &AttributeMask) {
        self.inner
            .write()
            .unwrap()
            .attributes_changed(light_root, attributes);
    }
}
