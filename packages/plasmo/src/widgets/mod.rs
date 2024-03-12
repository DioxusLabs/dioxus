mod button;
mod checkbox;
mod input;
mod number;
mod password;
mod slider;
mod text_like;
mod textbox;

use std::sync::{Arc, RwLock};

use dioxus_native_core::{
    custom_element::{CustomElement, CustomElementUpdater},
    real_dom::{NodeMut, RealDom},
};
use futures_channel::mpsc::UnboundedSender;
use shipyard::{Component, Unique};

use crate::Event;

pub(crate) fn register_widgets(rdom: &mut RealDom, sender: UnboundedSender<Event>) {
    // inject the widget context
    rdom.raw_world().add_unique(WidgetContext { sender });

    rdom.register_custom_element::<RinkWidgetWrapper<input::Input>>();
}

trait RinkWidget: Sync + Send + CustomElement + 'static {
    fn handle_event(&mut self, event: &Event, node: dioxus_native_core::real_dom::NodeMut);
}

pub trait RinkWidgetResponder: CustomElementUpdater {
    fn handle_event(&mut self, event: &Event, node: dioxus_native_core::real_dom::NodeMut);
}

impl<W: RinkWidget> RinkWidgetResponder for W {
    fn handle_event(&mut self, event: &Event, node: dioxus_native_core::real_dom::NodeMut) {
        RinkWidget::handle_event(self, event, node)
    }
}

struct RinkWidgetWrapper<W: RinkWidget> {
    inner: RinkWidgetTraitObject,
    _marker: std::marker::PhantomData<W>,
}

impl<W: RinkWidget> CustomElement for RinkWidgetWrapper<W> {
    const NAME: &'static str = W::NAME;

    const NAMESPACE: Option<&'static str> = W::NAMESPACE;

    fn create(mut node: NodeMut) -> Self {
        let myself = RinkWidgetTraitObject {
            widget: Arc::new(RwLock::new(W::create(node.reborrow()))),
        };

        // Insert the widget as an arbitrary data node so that it can be recognized when bubbling events
        node.insert(myself.clone());

        RinkWidgetWrapper {
            inner: myself,
            _marker: std::marker::PhantomData,
        }
    }

    fn attributes_changed(
        &mut self,
        root: dioxus_native_core::real_dom::NodeMut,
        attributes: &dioxus_native_core::node_ref::AttributeMask,
    ) {
        let mut widget = self.inner.widget.write().unwrap();
        widget.attributes_changed(root, attributes);
    }

    fn roots(&self) -> Vec<dioxus_native_core::NodeId> {
        let widget = self.inner.widget.read().unwrap();
        widget.roots()
    }

    fn slot(&self) -> Option<dioxus_native_core::NodeId> {
        let widget = self.inner.widget.read().unwrap();
        widget.slot()
    }
}

#[derive(Clone, Component)]
pub(crate) struct RinkWidgetTraitObject {
    widget: Arc<RwLock<dyn RinkWidgetResponder + Send + Sync>>,
}

impl CustomElementUpdater for RinkWidgetTraitObject {
    fn attributes_changed(
        &mut self,
        light_root: dioxus_native_core::real_dom::NodeMut,
        attributes: &dioxus_native_core::node_ref::AttributeMask,
    ) {
        let mut widget = self.widget.write().unwrap();
        widget.attributes_changed(light_root, attributes);
    }

    fn roots(&self) -> Vec<dioxus_native_core::NodeId> {
        let widget = self.widget.read().unwrap();
        widget.roots()
    }

    fn slot(&self) -> Option<dioxus_native_core::NodeId> {
        let widget = self.widget.read().unwrap();
        widget.slot()
    }
}

impl RinkWidgetResponder for RinkWidgetTraitObject {
    fn handle_event(&mut self, event: &Event, node: dioxus_native_core::real_dom::NodeMut) {
        let mut widget = self.widget.write().unwrap();
        widget.handle_event(event, node);
    }
}

#[derive(Unique, Clone)]
pub(crate) struct WidgetContext {
    sender: UnboundedSender<Event>,
}

impl WidgetContext {
    pub(crate) fn send(&self, event: Event) {
        self.sender.unbounded_send(event).unwrap();
    }
}
