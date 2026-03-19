use dioxus_core::{TemplateNode, VNode, VirtualDom};

#[derive(Default)]
pub(crate) struct HydrationValidationSession;

impl HydrationValidationSession {
    pub fn run_scope<E, F, P>(&mut self, _: Vec<web_sys::Node>, _: P, hydrate: F) -> Result<bool, E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
        P: FnOnce() -> Option<Vec<u32>>,
    {
        hydrate(self)?;
        Ok(false)
    }

    pub fn element<E, F>(
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

    pub fn text<E, F>(&mut self, _: &str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        hydrate(self)
    }

    pub fn placeholder<E, F>(&mut self, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        hydrate(self)
    }

    pub fn component<E, F>(&mut self, _: &'static str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        hydrate(self)
    }
}
