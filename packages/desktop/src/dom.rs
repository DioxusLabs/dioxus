//! webview dom

use dioxus_core::DomEdit;

// pub struct WebviewRegistry {}

// impl WebviewRegistry {
//     pub fn new() -> Self {
//         Self {}
//     }
// }

pub struct WebviewDom<'bump> {
    pub edits: Vec<DomEdit<'bump>>,
    pub node_counter: u64,
    // pub registry: WebviewRegistry,
}
impl WebviewDom<'_> {
    pub fn new() -> Self {
        Self {
            edits: Vec::new(),
            node_counter: 0,
            // registry,
        }
    }

    // // Finish using the dom (for its edit list) and give back the node and event registry
    // pub fn consume(self) -> WebviewRegistry {
    //     self.registry
    // }
}
