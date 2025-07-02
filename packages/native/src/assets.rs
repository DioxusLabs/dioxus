use blitz_shell::BlitzShellNetCallback;
use std::sync::Arc;

use blitz_dom::net::Resource;
use blitz_shell::BlitzShellEvent;
use blitz_traits::net::{NetCallback, NetProvider};
use winit::event_loop::EventLoopProxy;

pub struct DioxusNativeNetProvider {
    callback: Arc<dyn NetCallback<Resource> + 'static>,
    inner_net_provider: Arc<dyn NetProvider<Resource> + 'static>,
}
impl DioxusNativeNetProvider {
    pub fn shared(proxy: EventLoopProxy<BlitzShellEvent>) -> Arc<dyn NetProvider<Resource>> {
        Arc::new(Self::new(proxy)) as Arc<dyn NetProvider<Resource>>
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

impl NetProvider<Resource> for DioxusNativeNetProvider {
    fn fetch(
        &self,
        doc_id: usize,
        request: blitz_traits::net::Request,
        handler: blitz_traits::net::BoxedHandler<Resource>,
    ) {
        if request.url.scheme() == "dioxus" {
            match dioxus_asset_resolver::serve_asset(request.url.path()) {
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
