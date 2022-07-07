use crate::{
    templete::{TemplateNodeId, TextTemplateSegment},
    AttributeValue, Listener, VNode,
};

#[derive(Debug)]
pub(crate) struct DynamicNodeMapping<'b> {
    nodes: &'b [Option<TemplateNodeId>],
    text: &'b [&'b [TemplateNodeId]],
    attributes: &'b [&'b [(TemplateNodeId, usize)]],
}

impl<'b> DynamicNodeMapping<'b> {
    pub(crate) fn all_dynamic(&self) -> impl Iterator<Item = TemplateNodeId> + 'b {
        self.nodes
            .iter()
            .filter_map(|o| o.as_ref())
            .chain(self.text.iter().map(|ids| ids.iter()).flatten())
            .copied()
            .chain(
                self.attributes
                    .iter()
                    .map(|ids| ids.iter())
                    .flatten()
                    .map(|dynamic| dynamic.0),
            )
    }

    pub(crate) fn get_dynamic_nodes_for_node_index(&self, idx: usize) -> Option<TemplateNodeId> {
        self.nodes[idx]
    }

    pub(crate) fn get_dynamic_nodes_for_text_index(&self, idx: usize) -> &'b [TemplateNodeId] {
        self.text[idx]
    }

    pub(crate) fn get_dynamic_nodes_for_attribute_index(
        &self,
        idx: usize,
    ) -> &'b [(TemplateNodeId, usize)] {
        self.attributes[idx]
    }
}

pub(crate) struct TemplateContext<'b> {
    pub nodes: &'b [VNode<'b>],
    pub text_segments: &'b [&'b str],
    pub attributes: &'b [AttributeValue<'b>],
    /// The listeners must not change during the lifetime of the context, use a dynamic node if the listeners change
    pub listeners: &'b [Listener<'b>],
}

impl<'b> TemplateContext<'b> {
    pub(crate) fn resolve_text<TextSegments, Text>(&self, text: &TextSegments) -> String
    where
        TextSegments: AsRef<[TextTemplateSegment<Text>]>,
        Text: AsRef<str>,
    {
        let mut result = String::new();
        for seg in text.as_ref() {
            match seg {
                TextTemplateSegment::Static(s) => result += s.as_ref(),
                TextTemplateSegment::Dynamic(idx) => result += self.text_segments[*idx],
            }
        }
        result
    }

    pub(crate) fn resolve_attribute(&self, idx: usize) -> &'b AttributeValue<'b> {
        &self.attributes[idx]
    }

    pub(crate) fn resolve_listener(&self, idx: usize) -> &'b Listener<'b> {
        &self.listeners[idx]
    }
}
