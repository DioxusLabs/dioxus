#[cfg(feature = "hydrate")]
mod hydrate;

#[cfg(all(
    feature = "hydrate",
    feature = "debug-hydration-validation",
    debug_assertions
))]
pub(crate) mod validation;

#[cfg(feature = "hydrate")]
use dioxus_core::{TemplateNode, VNode, VirtualDom};

#[cfg(feature = "hydrate")]
pub(crate) trait HydrationSession {
    fn run_scope<E, F, P, R>(
        &mut self,
        roots: Vec<web_sys::Node>,
        suspense_path: P,
        expected_rsx: R,
        hydrate: F,
    ) -> Result<bool, E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
        P: FnOnce() -> Option<Vec<u32>>,
        R: FnOnce() -> Result<String, E>;

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
    fn run_scope<E, F, P, R>(
        &mut self,
        _: Vec<web_sys::Node>,
        _: P,
        _: R,
        hydrate: F,
    ) -> Result<bool, E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
        P: FnOnce() -> Option<Vec<u32>>,
        R: FnOnce() -> Result<String, E>,
    {
        hydrate(self)?;
        Ok(false)
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
pub(crate) fn serialize_vnode_subtree(dom: &VirtualDom, vnode: &VNode) -> String {
    #[cfg(all(feature = "debug-hydration-validation", debug_assertions))]
    {
        validation::serialize::serialize_vnode_subtree(dom, vnode)
    }

    #[cfg(not(all(feature = "debug-hydration-validation", debug_assertions)))]
    {
        let _ = dom;
        let _ = vnode;
        String::new()
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
