use std::fmt::Debug;

use crate::{custom_element::CustomElementManager, node::FromAnyValue, prelude::NodeRef, NodeId};

#[derive(Clone)]
pub struct ShadowDom<V: FromAnyValue> {
    shadow_root: NodeId,
    updater: CustomElementManager<V>,
}

impl<V: FromAnyValue + Send + Sync> ShadowDom<V> {
    pub fn new(updater: CustomElementManager<V>) -> Self {
        Self {
            shadow_root: updater.root(),
            updater,
        }
    }
}

impl<V: FromAnyValue> Debug for ShadowDom<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShadowDom")
            .field("shadow_root", &self.shadow_root)
            .finish()
    }
}
