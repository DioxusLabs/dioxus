mod button;
mod checkbox;
mod input;
mod number;
mod password;
mod slider;
mod textbox;

use dioxus_core::{ElementId, RenderReturn, Scope};
pub use input::*;

pub(crate) fn get_root_id<T>(cx: Scope<T>) -> Option<ElementId> {
    if let RenderReturn::Sync(Some(sync)) = cx.root_node() {
        sync.root_ids.get(0)
    } else {
        None
    }
}
