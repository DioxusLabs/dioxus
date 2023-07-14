//! A custom element is a controlled element that renders to a shadow DOM. This allows you to create elements that act like widgets without relying on a specific framework.
//!
//! Each custom element is registered with a element name and namespace with [`RealDom::register_custom_element`] or [`RealDom::register_custom_element_with_factory`]. Once registered, they will be created automatically when the element is added to the DOM.

// Used in doc links
#[allow(unused)]
use crate::real_dom::RealDom;

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
                let boxed_custom_element = { (builder.create)(node.reborrow()) };

                let shadow_roots = boxed_custom_element.roots();

                let light_id = node.id();
                node.real_dom_mut().tree_mut().create_subtree(
                    light_id,
                    shadow_roots,
                    boxed_custom_element.slot(),
                );

                let boxed_custom_element = CustomElementManager {
                    inner: Arc::new(RwLock::new(boxed_custom_element)),
                };

                node.insert(boxed_custom_element);
            }
        }
    }
}

struct CustomElementBuilder<V: FromAnyValue + Send + Sync> {
    create: fn(NodeMut<V>) -> Box<dyn CustomElementUpdater<V>>,
}

/// A controlled element that renders to a shadow DOM.
///
/// Register with [`RealDom::register_custom_element`]
///
/// This is a simplified custom element trait for elements that can create themselves. For more granular control, implement [`CustomElementFactory`] and [`CustomElementUpdater`] instead.
pub trait CustomElement<V: FromAnyValue + Send + Sync = ()>: Send + Sync + 'static {
    /// The tag of the element
    const NAME: &'static str;

    /// The namespace of the element
    const NAMESPACE: Option<&'static str> = None;

    /// Create a new element *without mounting* it.
    /// The node passed in is the light DOM node. The element should not modify the light DOM node, but it can get the [`NodeMut::real_dom_mut`] from the node to create new nodes.
    fn create(light_root: NodeMut<V>) -> Self;

    /// The root node of the custom element. These roots must be not change once the element is created.
    fn roots(&self) -> Vec<NodeId>;

    /// The slot to render children of the element into. The slot must be not change once the element is created.
    fn slot(&self) -> Option<NodeId> {
        None
    }

    /// Update the custom element's shadow tree with the new attributes.
    /// Called when the attributes of the custom element are changed.
    fn attributes_changed(&mut self, light_node: NodeMut<V>, attributes: &AttributeMask);
}

/// A factory for creating custom elements
///
/// Register with [`RealDom::register_custom_element_with_factory`]
pub trait CustomElementFactory<W: CustomElementUpdater<V>, V: FromAnyValue + Send + Sync = ()>:
    Send + Sync + 'static
{
    /// The tag of the element
    const NAME: &'static str;

    /// The namespace of the element
    const NAMESPACE: Option<&'static str> = None;

    /// Create a new element *without mounting* it.
    /// The node passed in is the light DOM node. The element should not modify the light DOM node, but it can get the [`NodeMut::real_dom_mut`] from the node to create new nodes.
    fn create(dom: NodeMut<V>) -> W;
}

impl<W: CustomElement<V>, V: FromAnyValue + Send + Sync> CustomElementFactory<W, V> for W {
    const NAME: &'static str = W::NAME;

    const NAMESPACE: Option<&'static str> = W::NAMESPACE;

    fn create(node: NodeMut<V>) -> Self {
        Self::create(node)
    }
}

/// A trait for updating custom elements
pub trait CustomElementUpdater<V: FromAnyValue + Send + Sync = ()>: Send + Sync + 'static {
    /// Update the custom element's shadow tree with the new attributes.
    /// Called when the attributes of the custom element are changed.
    fn attributes_changed(&mut self, light_root: NodeMut<V>, attributes: &AttributeMask);

    /// The root node of the custom element. These roots must be not change once the element is created.
    fn roots(&self) -> Vec<NodeId>;

    /// The slot to render children of the element into. The slot must be not change once the element is created.
    fn slot(&self) -> Option<NodeId> {
        None
    }
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

/// A dynamic trait object wrapper for [`CustomElementUpdater`]
#[derive(Component, Clone)]
pub(crate) struct CustomElementManager<V: FromAnyValue = ()> {
    inner: Arc<RwLock<Box<dyn CustomElementUpdater<V>>>>,
}

impl<V: FromAnyValue + Send + Sync> CustomElementManager<V> {
    /// Update the custom element based on attributes changed.
    pub fn on_attributes_changed(&self, light_root: NodeMut<V>, attributes: &AttributeMask) {
        self.inner
            .write()
            .unwrap()
            .attributes_changed(light_root, attributes);
    }
}
