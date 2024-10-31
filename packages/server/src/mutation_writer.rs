use dioxus_lib::prelude::dioxus_core::{Mutation, WriteMutations};

pub struct MutationWriter<F> {
    f: F,
}

impl<F: FnMut(Mutation)> WriteMutations for MutationWriter<F> {
    fn append_children(&mut self, id: dioxus_lib::prelude::dioxus_core::ElementId, m: usize) {
        todo!()
    }

    fn assign_node_id(
        &mut self,
        path: &'static [u8],
        id: dioxus_lib::prelude::dioxus_core::ElementId,
    ) {
        todo!()
    }

    fn create_placeholder(&mut self, id: dioxus_lib::prelude::dioxus_core::ElementId) {
        todo!()
    }

    fn create_text_node(&mut self, value: &str, id: dioxus_lib::prelude::dioxus_core::ElementId) {
        todo!()
    }

    fn load_template(
        &mut self,
        template: dioxus_lib::prelude::Template,
        index: usize,
        id: dioxus_lib::prelude::dioxus_core::ElementId,
    ) {
        todo!()
    }

    fn replace_node_with(&mut self, id: dioxus_lib::prelude::dioxus_core::ElementId, m: usize) {
        todo!()
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        todo!()
    }

    fn insert_nodes_after(&mut self, id: dioxus_lib::prelude::dioxus_core::ElementId, m: usize) {
        todo!()
    }

    fn insert_nodes_before(&mut self, id: dioxus_lib::prelude::dioxus_core::ElementId, m: usize) {
        todo!()
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &dioxus_lib::prelude::dioxus_core::AttributeValue,
        id: dioxus_lib::prelude::dioxus_core::ElementId,
    ) {
        todo!()
    }

    fn set_node_text(&mut self, value: &str, id: dioxus_lib::prelude::dioxus_core::ElementId) {
        todo!()
    }

    fn create_event_listener(
        &mut self,
        name: &'static str,
        id: dioxus_lib::prelude::dioxus_core::ElementId,
    ) {
        todo!()
    }

    fn remove_event_listener(
        &mut self,
        name: &'static str,
        id: dioxus_lib::prelude::dioxus_core::ElementId,
    ) {
        todo!()
    }

    fn remove_node(&mut self, id: dioxus_lib::prelude::dioxus_core::ElementId) {
        todo!()
    }

    fn push_root(&mut self, id: dioxus_lib::prelude::dioxus_core::ElementId) {
        todo!()
    }
}
