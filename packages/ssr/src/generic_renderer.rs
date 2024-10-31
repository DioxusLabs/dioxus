use std::fmt::Write;

use dioxus_core::{Template, VNode, VirtualDom};
use rustc_hash::FxHashMap;

use crate::cache::StringCache;

struct Cache {
    items: FxHashMap<Template, StringCache>,
}

// trait Renderer {
//     fn cache(&mut self, template: &Template) -> &StringCache;

//     fn render_template(&mut self, buf: &mut impl Write, template: &Template) -> std::fmt::Result {
//         let entry = self.cache(template);

//         let mut inner_html = None;

//         // We need to keep track of the dynamic styles so we can insert them into the right place
//         let mut accumulated_dynamic_styles = Vec::new();

//         // We need to keep track of the listeners so we can insert them into the right place
//         let mut accumulated_listeners = Vec::new();

//         // We keep track of the index we are on manually so that we can jump forward to a new section quickly without iterating every item
//         let mut index = 0;
//         let mut dynamic_node_id = 0;

//         todo!()
//     }
// }
