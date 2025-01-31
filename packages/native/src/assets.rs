use blitz_shell::BlitzShellNetCallback;
use std::{path::PathBuf, sync::Arc};

use blitz_dom::net::Resource;
use blitz_shell::BlitzShellEvent;
use blitz_traits::net::{NetCallback, NetProvider};
use winit::event_loop::EventLoopProxy;

pub struct DioxusNativeNetProvider {
    proxy: EventLoopProxy<BlitzShellEvent>,
    callback: Arc<dyn NetCallback<Data = Resource> + 'static>,
    inner_net_provider: Arc<dyn NetProvider<Data = Resource> + 'static>,
}
impl DioxusNativeNetProvider {
    pub fn shared(proxy: EventLoopProxy<BlitzShellEvent>) -> Arc<dyn NetProvider<Data = Resource>> {
        Arc::new(Self::new(proxy)) as Arc<dyn NetProvider<Data = Resource>>
    }

    pub fn new(proxy: EventLoopProxy<BlitzShellEvent>) -> Self {
        let net_callback = BlitzShellNetCallback::shared(proxy.clone());
        let net_provider = blitz_net::Provider::shared(net_callback.clone());
        Self {
            proxy,
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
        use blitz_traits::net::NetCallback;
        use blitz_traits::net::NetHandler;

        if request.url.scheme() == "dioxus" {
            match dioxus_asset_resolver::serve_asset_from_raw_path(&request.url.path()) {
                Ok(res) => {
                    println!("fetching asset  from file system success {request:#?}");
                    handler.bytes(doc_id, res.into_body().into(), self.callback.clone())
                }
                Err(_) => {
                    println!("fetching asset  from file system error {request:#?}");
                }
            }
        } else {
            self.inner_net_provider.fetch(doc_id, request, handler);
        }
    }
}
