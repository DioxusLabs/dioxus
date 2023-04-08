use std::sync::{Arc, RwLock};

use rustc_hash::FxHashMap;
use shipyard::Component;

use crate::{
    node::{FromAnyValue, NodeType},
    node_ref::AttributeMask,
    prelude::{NodeImmutable, NodeMut, RealDom},
    NodeId,
};

pub(crate) struct CustomElementRegistry<V: FromAnyValue + Send + Sync> {
    builders: FxHashMap<&'static str, CustomElementBuilder<V>>,
}

impl<V: FromAnyValue + Send + Sync> Default for CustomElementRegistry<V> {
    fn default() -> Self {
        Self {
            builders: FxHashMap::default(),
        }
    }
}

impl<V: FromAnyValue + Send + Sync> CustomElementRegistry<V> {
    pub fn register<W: CustomElement<V>>(&mut self) {
        self.builders.insert(
            W::NAME,
            CustomElementBuilder {
                create: |dom| Box::new(W::create(dom)),
            },
        );
    }

    pub fn add_shadow_dom(&self, mut node: NodeMut<V>) {
        let element_tag = if let NodeType::Element(el) = &*node.node_type() {
            Some(el.tag.clone())
        } else {
            None
        };
        if let Some(element_tag) = element_tag {
            if let Some(builder) = self.builders.get(element_tag.as_str()) {
                let boxed_widget = {
                    let dom = node.real_dom_mut();
                    (builder.create)(dom)
                };

                let boxed_widget = CustomElementManager {
                    inner: Arc::new(RwLock::new(boxed_widget)),
                };

                node.insert(boxed_widget);
            }
        }
    }
}

struct CustomElementBuilder<V: FromAnyValue + Send + Sync> {
    create: fn(&mut RealDom<V>) -> Box<dyn CustomElementUpdater<V>>,
}

/// A controlled element that renders to a shadow DOM
pub trait CustomElement<V: FromAnyValue + Send + Sync = ()>: Send + Sync + 'static {
    /// The tag the widget is registered under.
    const NAME: &'static str;

    /// Create a new widget without mounting it.
    fn create(dom: &mut RealDom<V>) -> Self;

    /// The root node of the widget. This must be static once the element is created.
    fn root(&self) -> NodeId;

    /// The slot to render children of the element into. This must be static once the element is created.
    fn slot(&self) -> Option<NodeId> {
        None
    }

    /// Called when the attributes of the widget are changed.
    fn attributes_changed(&mut self, _dom: &mut RealDom<V>, _attributes: &AttributeMask);
}

/// A factory for creating widgets
trait ElementFactory<W: CustomElementUpdater<V>, V: FromAnyValue + Send + Sync = ()>:
    Send + Sync + 'static
{
    /// The tag the widget is registered under.
    const NAME: &'static str;

    /// Create a new widget.
    fn create(dom: &mut RealDom<V>) -> W;
}

impl<W: CustomElement<V>, V: FromAnyValue + Send + Sync> ElementFactory<W, V> for W {
    const NAME: &'static str = W::NAME;

    fn create(dom: &mut RealDom<V>) -> Self {
        Self::create(dom)
    }
}

/// A trait for updating widgets
trait CustomElementUpdater<V: FromAnyValue + Send + Sync = ()>: Send + Sync + 'static {
    /// Called when the attributes of the widget are changed.
    fn attributes_changed(&mut self, dom: &mut RealDom<V>, attributes: &AttributeMask);

    /// The root node of the widget.
    fn root(&self) -> NodeId;
}

impl<W: CustomElement<V>, V: FromAnyValue + Send + Sync> CustomElementUpdater<V> for W {
    fn attributes_changed(&mut self, root: &mut RealDom<V>, attributes: &AttributeMask) {
        self.attributes_changed(root, attributes);
    }

    fn root(&self) -> NodeId {
        self.root()
    }
}

#[derive(Component, Clone)]
pub struct CustomElementManager<V: FromAnyValue = ()> {
    inner: Arc<RwLock<Box<dyn CustomElementUpdater<V>>>>,
}

impl<V: FromAnyValue + Send + Sync> CustomElementManager<V> {
    pub fn root(&self) -> NodeId {
        self.inner.read().unwrap().root()
    }

    pub fn on_attributes_changed(&self, dom: &mut RealDom<V>, attributes: &AttributeMask) {
        self.inner
            .write()
            .unwrap()
            .attributes_changed(dom, attributes);
    }
}
