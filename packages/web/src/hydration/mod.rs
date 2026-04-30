#[cfg(feature = "hydrate")]
mod hydrate;

#[cfg(all(
    feature = "hydrate",
    feature = "debug-hydration-validation",
    debug_assertions
))]
pub(crate) mod validation;

#[cfg(feature = "hydrate")]
use crate::dom::WebsysDom;
#[cfg(feature = "hydrate")]
use dioxus_core::{ScopeId, TemplateNode, VNode, VirtualDom};
#[cfg(feature = "hydrate")]
use hydrate::{finalize_hydrate, HydrationOutputs, RehydrationError};

#[cfg(feature = "hydrate")]
pub(crate) trait HydrationSession {
    type RecoveryAnchor;

    fn root_recovery(&self) -> Self::RecoveryAnchor;

    fn streaming_recovery(&self, anchor: &web_sys::Element) -> Self::RecoveryAnchor;

    /// Run the full hydration pass for one scope: walk the vnode (via
    /// `hydrate`), validate against the DOM, then either finalize the match
    /// or fall back to a client rebuild. The session owns the whole flow so
    /// the caller doesn't need to dispatch on outcome.
    fn run_scope<E, F>(
        &mut self,
        websys: &mut WebsysDom,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        under: Vec<web_sys::Node>,
        recovery: Self::RecoveryAnchor,
        suspense_path: Option<Vec<u32>>,
        hydrate: F,
    ) -> Result<(), E>
    where
        E: From<RehydrationError>,
        F: FnOnce(&mut WebsysDom, &mut VirtualDom, &mut Self) -> Result<HydrationOutputs, E>;

    fn element<E, F>(
        &mut self,
        dom: &VirtualDom,
        vnode: &VNode,
        node: &TemplateNode,
        hydrate: F,
    ) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>;

    fn text<E, F>(&mut self, expected_content: &str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>;

    fn placeholder<E, F>(&mut self, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>;

    fn component<E, F>(&mut self, name: &'static str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>;
}

#[cfg(feature = "hydrate")]
#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct NoopHydrationSession;

#[cfg(feature = "hydrate")]
impl HydrationSession for NoopHydrationSession {
    type RecoveryAnchor = ();

    fn root_recovery(&self) {}
    fn streaming_recovery(&self, _: &web_sys::Element) {}

    fn run_scope<E, F>(
        &mut self,
        websys: &mut WebsysDom,
        dom: &mut VirtualDom,
        _: ScopeId,
        under: Vec<web_sys::Node>,
        _: Self::RecoveryAnchor,
        _: Option<Vec<u32>>,
        hydrate: F,
    ) -> Result<(), E>
    where
        E: From<RehydrationError>,
        F: FnOnce(&mut WebsysDom, &mut VirtualDom, &mut Self) -> Result<HydrationOutputs, E>,
    {
        let outputs = hydrate(websys, dom, self)?;
        finalize_hydrate(websys, outputs, under);
        Ok(())
    }

    fn element<E, F>(
        &mut self,
        _: &VirtualDom,
        _: &VNode,
        _: &TemplateNode,
        hydrate: F,
    ) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        hydrate(self)
    }

    fn text<E, F>(&mut self, _: &str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        hydrate(self)
    }

    fn placeholder<E, F>(&mut self, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        hydrate(self)
    }

    fn component<E, F>(&mut self, _: &'static str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        hydrate(self)
    }
}

#[cfg(feature = "hydrate")]
pub(crate) fn session() -> impl HydrationSession {
    #[cfg(all(feature = "debug-hydration-validation", debug_assertions))]
    {
        validation::HydrationValidationSession::default()
    }

    #[cfg(not(all(feature = "debug-hydration-validation", debug_assertions)))]
    {
        NoopHydrationSession
    }
}

#[cfg(feature = "hydrate")]
#[allow(unused)]
pub use hydrate::*;

/// The message sent from the server to the client to hydrate a suspense boundary
#[derive(Debug)]
pub(crate) struct SuspenseMessage {
    #[cfg(feature = "hydrate")]
    /// The path to the suspense boundary. Each element in the path is an index into the children of the suspense boundary (or the root node) in the order they are first created
    suspense_path: Vec<u32>,
    #[cfg(feature = "hydrate")]
    /// The data to hydrate the suspense boundary with
    data: Vec<u8>,
    #[cfg(feature = "hydrate")]
    #[cfg(debug_assertions)]
    /// The type names of the data
    debug_types: Option<Vec<String>>,
    #[cfg(feature = "hydrate")]
    #[cfg(debug_assertions)]
    /// The location of the data in the source code
    debug_locations: Option<Vec<String>>,
}
