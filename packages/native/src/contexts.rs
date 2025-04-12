use std::{cell::RefCell, ops::Deref, rc::Rc, sync::Arc};

use blitz_renderer_vello::BlitzVelloRenderer;
use blitz_shell::BlitzShellEvent;
use blitz_traits::BlitzWindowHandle;
use dioxus_document::{Document, NoOpDocument};
use peniko::Blob;
use winit::{
    event_loop::EventLoopProxy,
    window::{Window, WindowId},
};

use crate::{DioxusNativeEvent, ReservedNativeTexture, SharedNativeTexture};

pub struct NativeDocument {
    pub(crate) proxy: EventLoopProxy<BlitzShellEvent>,
    pub(crate) renderer: Rc<RefCell<BlitzVelloRenderer>>,
    pub(crate) window_id: WindowId,
}

impl NativeDocument {
    pub(crate) fn new(
        proxy: EventLoopProxy<BlitzShellEvent>,
        window: WindowId,
        renderer: Rc<RefCell<BlitzVelloRenderer>>,
    ) -> Self {
        Self {
            proxy,
            renderer,
            window_id: window,
        }
    }

    pub fn window_handle(&self) -> Arc<dyn BlitzWindowHandle> {
        self.renderer.borrow().window_handle.clone()
    }

    pub fn set_custom_texture(&self, node_id: &str, texture: SharedNativeTexture) {
        let mut renderer = self.renderer.borrow_mut();

        if let Some(image) = renderer.custom_textures.get(node_id).cloned() {
            let blitz_renderer_vello::RenderState::Active(state) = &mut renderer.render_state
            else {
                return;
            };

            state
                .renderer
                .override_image(&image, Some(texture.inner.clone()));
        }

        // if let Some(node_id) = window.doc.inner.nodes_to_id.get(node_id).cloned() {
        //     match &window.doc.inner.nodes[node_id].data {
        //         blitz_dom::NodeData::Element(data) => match &data.node_specific_data {
        //             blitz_dom::node::NodeSpecificData::Image(image_data) => {
        //                 match image_data.as_ref() {
        //                     blitz_dom::node::ImageData::CustomTexture(image) => {
        //                         state
        //                             .renderer
        //                             .override_image(&image, Some(texture.inner.clone()));
        //                     }
        //                     _ => {}
        //                 }
        //             }
        //             _ => {}
        //         },
        //         _ => {}
        //     }
        // }
    }
}

impl Document for NativeDocument {
    fn eval(&self, _js: String) -> dioxus_document::Eval {
        NoOpDocument.eval(_js)
    }

    fn create_head_element(
        &self,
        name: &str,
        attributes: &[(&str, String)],
        contents: Option<String>,
    ) {
        let window = self.window_id;
        _ = self.proxy.send_event(BlitzShellEvent::embedder_event(
            DioxusNativeEvent::CreateHeadElement {
                name: name.to_string(),
                attributes: attributes
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.clone()))
                    .collect(),
                contents,
                window,
            },
        ));
    }

    fn set_title(&self, title: String) {
        self.create_head_element("title", &[], Some(title));
    }

    fn create_meta(&self, props: dioxus_document::MetaProps) {
        let attributes = props.attributes();
        self.create_head_element("meta", &attributes, None);
    }

    fn create_script(&self, props: dioxus_document::ScriptProps) {
        let attributes = props.attributes();
        self.create_head_element("script", &attributes, props.script_contents().ok());
    }

    fn create_style(&self, props: dioxus_document::StyleProps) {
        let attributes = props.attributes();
        self.create_head_element("style", &attributes, props.style_contents().ok());
    }

    fn create_link(&self, props: dioxus_document::LinkProps) {
        let attributes = props.attributes();
        self.create_head_element("link", &attributes, None);
    }

    fn create_head_component(&self) -> bool {
        true
    }
}
