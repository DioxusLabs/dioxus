use crate::edits::EditQueue;
use crate::DesktopContext;
use dioxus_core::ScopeState;
use slab::Slab;
use std::{
    borrow::Cow,
    future::Future,
    ops::Deref,
    path::{Path, PathBuf},
    pin::Pin,
    rc::Rc,
    sync::Arc,
};
use tokio::{
    runtime::Handle,
    sync::{OnceCell, RwLock},
};
use wry::{
    http::{status::StatusCode, Request, Response},
    Result,
};

/// An arbitrary asset is an HTTP response containing a binary body.
pub type AssetResponse = Response<Cow<'static, [u8]>>;

/// A future that returns an [`AssetResponse`]. This future may be spawned in a new thread,
/// so it must be [`Send`], [`Sync`], and `'static`.
pub trait AssetFuture: Future<Output = Option<AssetResponse>> + Send + Sync + 'static {}
impl<T: Future<Output = Option<AssetResponse>> + Send + Sync + 'static> AssetFuture for T {}

#[derive(Debug, Clone)]
/// A request for an asset. This is a wrapper around [`Request<Vec<u8>>`] that provides methods specific to asset requests.
pub struct AssetRequest {
    pub(crate) path: PathBuf,
    pub(crate) request: Arc<Request<Vec<u8>>>,
}

impl AssetRequest {
    /// Get the path the asset request is for
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl From<Request<Vec<u8>>> for AssetRequest {
    fn from(request: Request<Vec<u8>>) -> Self {
        let decoded = urlencoding::decode(request.uri().path().trim_start_matches('/'))
            .expect("expected URL to be UTF-8 encoded");
        let path = PathBuf::from(&*decoded);
        Self {
            request: Arc::new(request),
            path,
        }
    }
}

impl Deref for AssetRequest {
    type Target = Request<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

/// A handler that takes an [`AssetRequest`] and returns a future that either loads the asset, or returns `None`.
/// This handler is stashed indefinitely in a context object, so it must be `'static`.
pub trait AssetHandler<F: AssetFuture>: Send + Sync + 'static {
    /// Handle an asset request, returning a future that either loads the asset, or returns `None`
    fn handle_request(&self, request: &AssetRequest) -> F;
}

impl<F: AssetFuture, T: Fn(&AssetRequest) -> F + Send + Sync + 'static> AssetHandler<F> for T {
    fn handle_request(&self, request: &AssetRequest) -> F {
        self(request)
    }
}

type UserAssetHandler =
    Box<dyn Fn(&AssetRequest) -> Pin<Box<dyn AssetFuture>> + Send + Sync + 'static>;

type AssetHandlerRegistryInner = Slab<UserAssetHandler>;

#[derive(Clone)]
pub struct AssetHandlerRegistry(Arc<RwLock<AssetHandlerRegistryInner>>);

impl AssetHandlerRegistry {
    pub fn new() -> Self {
        AssetHandlerRegistry(Arc::new(RwLock::new(Slab::new())))
    }

    pub async fn register_handler<F: AssetFuture>(&self, f: impl AssetHandler<F>) -> usize {
        let mut registry = self.0.write().await;
        registry.insert(Box::new(move |req| Box::pin(f.handle_request(req))))
    }

    pub async fn remove_handler(&self, id: usize) -> Option<()> {
        let mut registry = self.0.write().await;
        registry.try_remove(id).map(|_| ())
    }

    pub async fn try_handlers(&self, req: &AssetRequest) -> Option<AssetResponse> {
        let registry = self.0.read().await;
        for (_, handler) in registry.iter() {
            if let Some(response) = handler(req).await {
                return Some(response);
            }
        }
        None
    }
}

/// A handle to a registered asset handler.
pub struct AssetHandlerHandle {
    pub(crate) desktop: DesktopContext,
    pub(crate) handler_id: Rc<OnceCell<usize>>,
}

impl AssetHandlerHandle {
    /// Returns the ID for this handle.
    ///
    /// Because registering an ID is asynchronous, this may return `None` if the
    /// registration has not completed yet.
    pub fn handler_id(&self) -> Option<usize> {
        self.handler_id.get().copied()
    }
}

impl Drop for AssetHandlerHandle {
    fn drop(&mut self) {
        let cell = Rc::clone(&self.handler_id);
        let desktop = Rc::clone(&self.desktop);
        tokio::task::block_in_place(move || {
            Handle::current().block_on(async move {
                if let Some(id) = cell.get() {
                    desktop.asset_handlers.remove_handler(*id).await;
                }
            })
        });
    }
}
