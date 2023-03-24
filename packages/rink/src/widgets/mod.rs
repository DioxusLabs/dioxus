mod button;
mod checkbox;
mod input;
mod number;
mod password;
mod slider;
mod textbox;

use std::sync::{Arc, RwLock};

use dioxus_native_core::{
    real_dom::RealDom,
    utils::widget_watcher::{Widget, WidgetFactory, WidgetUpdater, WidgetWatcher},
};
use futures_channel::mpsc::UnboundedSender;
use shipyard::{Component, Unique};

use crate::Event;

pub(crate) fn register_widgets(rdom: &mut RealDom, sender: UnboundedSender<Event>) {
    // inject the widget context
    rdom.raw_world().add_unique(WidgetContext { sender });

    // create the widget watcher
    let mut widget_watcher = WidgetWatcher::default();

    widget_watcher
        .register_widget::<RinkWidgetTraitObjectFactory<input::Input>, RinkWidgetTraitObject>();

    widget_watcher.attach(rdom);
}

trait RinkWidget: Sync + Send + Widget + 'static {
    fn handle_event(&mut self, event: &Event, node: &mut dioxus_native_core::real_dom::NodeMut);
}

pub trait RinkWidgetResponder: WidgetUpdater {
    fn handle_event(&mut self, event: &Event, node: &mut dioxus_native_core::real_dom::NodeMut);
}

impl<W: RinkWidget> RinkWidgetResponder for W {
    fn handle_event(&mut self, event: &Event, node: &mut dioxus_native_core::real_dom::NodeMut) {
        RinkWidget::handle_event(self, event, node)
    }
}

struct RinkWidgetTraitObjectFactory<W: RinkWidget> {
    _marker: std::marker::PhantomData<W>,
}

impl<W: RinkWidget> WidgetFactory<RinkWidgetTraitObject> for RinkWidgetTraitObjectFactory<W> {
    const NAME: &'static str = W::NAME;

    fn create(node: &mut dioxus_native_core::real_dom::NodeMut) -> RinkWidgetTraitObject {
        let myself = RinkWidgetTraitObject {
            widget: Arc::new(RwLock::new(W::create(node))),
        };
        node.insert(myself.clone());
        myself
    }
}

#[derive(Clone, Component)]
pub(crate) struct RinkWidgetTraitObject {
    widget: Arc<RwLock<dyn RinkWidgetResponder + Send + Sync>>,
}

impl WidgetUpdater for RinkWidgetTraitObject {
    fn attributes_changed(
        &mut self,
        root: dioxus_native_core::real_dom::NodeMut,
        attributes: &dioxus_native_core::node_ref::AttributeMask,
    ) {
        let mut widget = self.widget.write().unwrap();
        widget.attributes_changed(root, attributes);
    }
}

impl RinkWidgetResponder for RinkWidgetTraitObject {
    fn handle_event(&mut self, event: &Event, node: &mut dioxus_native_core::real_dom::NodeMut) {
        let mut widget = self.widget.write().unwrap();
        widget.handle_event(event, node);
    }
}

#[derive(Unique)]
pub(crate) struct WidgetContext {
    sender: UnboundedSender<Event>,
}

impl WidgetContext {
    pub(crate) fn send(&self, event: Event) {
        self.sender.unbounded_send(event).unwrap();
    }
}
