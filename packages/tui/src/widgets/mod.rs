mod button;
mod checkbox;
mod input;
mod number;
mod password;
mod slider;
mod textbox;

use dioxus_core::{RenderReturn, Scope};
use dioxus_native_core::NodeId;
pub use input::*;

use crate::DioxusElementToNodeId;

pub(crate) fn get_root_id<T>(cx: Scope<T>) -> Option<NodeId> {
    if let RenderReturn::Ready(sync) = cx.root_node() {
        let mapping: DioxusElementToNodeId = cx.consume_context()?;
        mapping.get_node_id(sync.root_ids.get(0)?)
    } else {
        None
    }
}
