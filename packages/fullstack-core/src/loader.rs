use dioxus_core::{CapturedError, RenderError, Result};
use dioxus_hooks::Resource;
use dioxus_signals::{MappedMutSignal, WriteSignal};
use dioxus_stores::MappedStore;
use serde::{de::DeserializeOwned, Serialize};
use std::{cmp::PartialEq, future::Future};

use crate::use_server_future;

/// A hook to create a resource that loads data asynchronously.
///
/// This hook takes a closure that returns a future. This future will be executed on both the client
/// and the server. The loader will return `Loading` until the future resolves, at which point it will
/// return a `Loader<T>`. If the future fails, it will return `Loading::Failed`.
///
/// After the loader has successfully loaded once, it will never suspend the component again, but will
/// instead re-load the value in the background whenever any of its dependencies change.
///
/// If an error occurs while re-loading, `use_loader` will once again emit a `Loading::Failed` value.
/// The `use_loader` hook will never return a suspended state after the initial load.
///
/// # On the server
///
/// On the server, this hook will block the rendering of the component (and therefore, the page) until
/// the future resolves. Any server futures called by `use_loader` will receive the same request context
/// as the component that called `use_loader`.
#[allow(clippy::result_large_err)]
#[track_caller]
pub fn use_loader<F, T, E>(
    mut future: impl FnMut() -> F + 'static,
) -> Result<
    Resource<
        MappedStore<
            T,
            MappedMutSignal<
                Result<T, CapturedError>,
                WriteSignal<Option<Result<T, CapturedError>>>,
            >,
        >,
    >,
    RenderError,
>
where
    F: Future<Output = Result<T, E>> + 'static,
    T: 'static + PartialEq + Serialize + DeserializeOwned,
    E: Into<dioxus_core::Error> + 'static,
{
    let resolved = use_server_future(move || {
        let fut = future();
        async move { fut.await.map_err(|e| CapturedError::from(e.into())) }
    })?;
    let ok = resolved.transpose().map_err(|e| RenderError::Error(e()))?;
    Ok(ok)
}
