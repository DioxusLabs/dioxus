use blitz_shell::BlitzShellNetCallback;
use std::sync::Arc;

use blitz_dom::net::Resource;
use blitz_shell::BlitzShellEvent;
use blitz_traits::net::{NetCallback, NetProvider};
use winit::event_loop::EventLoopProxy;

pub struct DioxusNativeNetProvider {
    callback: Arc<dyn NetCallback<Resource> + 'static>,
    #[cfg(feature = "net")]
    inner_net_provider: Arc<dyn NetProvider<Resource> + 'static>,
}
impl DioxusNativeNetProvider {
    pub fn shared(proxy: EventLoopProxy<BlitzShellEvent>) -> Arc<dyn NetProvider<Resource>> {
        Arc::new(Self::new(proxy)) as Arc<dyn NetProvider<Resource>>
    }

    pub fn new(proxy: EventLoopProxy<BlitzShellEvent>) -> Self {
        let net_callback = BlitzShellNetCallback::shared(proxy);

        #[cfg(feature = "net")]
        let net_provider = blitz_net::Provider::shared(net_callback.clone());

        Self {
            callback: net_callback,
            #[cfg(feature = "net")]
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
            match dioxus_asset_resolver::native::serve_asset(request.url.path()) {
                Ok(res) => {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("fetching asset from file system success {request:#?}");
                    handler.bytes(doc_id, res.into_body().into(), self.callback.clone())
                }
                Err(_) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("fetching asset from file system error {request:#?}");
                }
            }
        } else {
            #[cfg(feature = "net")]
            self.inner_net_provider.fetch(doc_id, request, handler);

            #[cfg(all(not(feature = "net"), feature = "tracing"))]
            tracing::warn!("net feature not enabled, cannot fetch {request:#?}");
        }
    }
}
