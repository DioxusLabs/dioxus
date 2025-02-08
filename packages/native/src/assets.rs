use blitz_shell::BlitzShellNetCallback;
use std::sync::Arc;

use blitz_dom::{
    net::{CssHandler, ImageHandler, Resource},
    util::ImageType,
    BaseDocument,
};
use blitz_shell::BlitzShellEvent;
use blitz_traits::net::{NetCallback, NetProvider};
use winit::event_loop::EventLoopProxy;

use crate::NodeId;

pub struct DioxusNativeNetProvider {
    callback: Arc<dyn NetCallback<Data = Resource> + 'static>,
    inner_net_provider: Arc<dyn NetProvider<Data = Resource> + 'static>,
}
impl DioxusNativeNetProvider {
    pub fn shared(proxy: EventLoopProxy<BlitzShellEvent>) -> Arc<dyn NetProvider<Data = Resource>> {
        Arc::new(Self::new(proxy)) as Arc<dyn NetProvider<Data = Resource>>
    }

    pub fn new(proxy: EventLoopProxy<BlitzShellEvent>) -> Self {
        let net_callback = BlitzShellNetCallback::shared(proxy);
        let net_provider = blitz_net::Provider::shared(net_callback.clone());
        Self {
            callback: net_callback,
            inner_net_provider: net_provider,
        }
    }
}

impl NetProvider for DioxusNativeNetProvider {
    type Data = Resource;

    fn fetch(
        &self,
        doc_id: usize,
        request: blitz_traits::net::Request,
        handler: blitz_traits::net::BoxedHandler<Self::Data>,
    ) {
        if request.url.scheme() == "dioxus" {
            match dioxus_asset_resolver::serve_asset_from_raw_path(request.url.path()) {
                Ok(res) => {
                    tracing::trace!("fetching asset  from file system success {request:#?}");
                    handler.bytes(doc_id, res.into_body().into(), self.callback.clone())
                }
                Err(_) => {
                    tracing::warn!("fetching asset  from file system error {request:#?}");
                }
            }
        } else {
            self.inner_net_provider.fetch(doc_id, request, handler);
        }
    }
}

pub fn fetch_linked_stylesheet(doc: &BaseDocument, node_id: NodeId, queued_url: String) {
    let url = doc.resolve_url(&queued_url);
    doc.net_provider.fetch(
        doc.id(),
        blitz_traits::net::Request::get(url.clone()),
        Box::new(CssHandler {
            node: node_id,
            source_url: url,
            guard: doc.guard.clone(),
            provider: doc.net_provider.clone(),
        }),
    );
}

pub fn fetch_image(doc: &BaseDocument, node_id: usize, queued_image: String) {
    let src = doc.resolve_url(&queued_image);
    doc.net_provider.fetch(
        doc.id(),
        blitz_traits::net::Request::get(src),
        Box::new(ImageHandler::new(node_id, ImageType::Image)),
    );
}
