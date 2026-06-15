use crate::attribute::Attribute;
use crate::{
    AttributeValue, BodyNode, HotLiteral, HotReloadFormattedSegment, Segment, TemplateBody,
};

/// A visitor that assigns dynamic ids to nodes and attributes and accumulates cursors to dynamic nodes and attributes
struct DynIdVisitor<'a> {
    body: &'a mut TemplateBody,
    current_cursor: Vec<u8>,
    dynamic_text_index: usize,
    component_literal_index: usize,
}

impl<'a> DynIdVisitor<'a> {
    fn new(body: &'a mut TemplateBody) -> Self {
        Self {
            body,
            current_cursor: Vec::new(),
            dynamic_text_index: 0,
            component_literal_index: 0,
        }
    }

    fn visit_children(&mut self, children: &[BodyNode]) {
        let mut static_idx = 0u8;
        for node in children {
            self.current_cursor.push(static_idx);
            self.visit(node);
            self.current_cursor.pop();
            if Self::is_static_template_node(node) {
                static_idx += 1;
            }
        }
    }

    fn is_static_template_node(node: &BodyNode) -> bool {
        match node {
            BodyNode::Element(_) => true,
            BodyNode::Text(text) => text.is_static(),
            _ => false,
        }
    }

    fn visit(&mut self, node: &BodyNode) {
        match node {
            // Just descend into elements - they're not dynamic
            BodyNode::Element(el) => {
                for (idx, attr) in el.merged_attributes.iter().enumerate() {
                    if !attr.is_static_str_literal() {
                        self.assign_path_to_attribute(attr, idx);
                        if let AttributeValue::AttrLiteral(HotLiteral::Fmted(lit)) = &attr.value {
                            self.assign_formatted_segment(lit);
                        }
                    }
                }
                // Assign formatted segments to the key which is not included in the merged_attributes
                if let Some(AttributeValue::AttrLiteral(HotLiteral::Fmted(fmted))) = el.key() {
                    self.assign_formatted_segment(fmted);
                }

                self.visit_children(&el.children);
            }

            // Text nodes are dynamic if they contain dynamic segments
            BodyNode::Text(txt) => {
                if !txt.is_static() {
                    self.assign_cursor_to_node(node);
                    self.assign_formatted_segment(&txt.input);
                }
            }

            // Raw exprs are always dynamic
            BodyNode::RawExpr(_) | BodyNode::ForLoop(_) | BodyNode::IfChain(_) => {
                self.assign_cursor_to_node(node)
            }
            BodyNode::Component(component) => {
                self.assign_cursor_to_node(node);
                let mut index = 0;
                for property in &component.fields {
                    if let AttributeValue::AttrLiteral(literal) = &property.value {
                        if let HotLiteral::Fmted(segments) = literal {
                            self.assign_formatted_segment(segments);
                        }
                        // Don't include keys in the component dynamic pool
                        if !property.name.is_likely_key() {
                            component.component_literal_dyn_idx[index]
                                .set(self.component_literal_index);
                            self.component_literal_index += 1;
                            index += 1;
                        }
                    }
                }
            }
        };
    }

    /// Assign ids to a formatted segment
    fn assign_formatted_segment(&mut self, segments: &HotReloadFormattedSegment) {
        let mut dynamic_node_indexes = segments.dynamic_node_indexes.iter();
        for segment in &segments.segments {
            if let Segment::Formatted(segment) = segment {
                dynamic_node_indexes
                    .next()
                    .unwrap()
                    .set(self.dynamic_text_index);
                self.dynamic_text_index += 1;
                self.body.dynamic_text_segments.push(segment.clone());
            }
        }
    }

    /// Assign a cursor to a node and give it its dynamic index
    /// This simplifies the ToTokens implementation for the macro to be a little less centralized
    fn assign_cursor_to_node(&mut self, node: &BodyNode) {
        node.set_dyn_idx(self.body.node_cursors.len());

        // And then save the current cursor as the corresponding cursor
        self.body.node_cursors.push(self.current_cursor.clone());
    }

    /// Assign a cursor to a attribute and give it its dynamic index
    /// This simplifies the ToTokens implementation for the macro to be a little less centralized
    pub(crate) fn assign_path_to_attribute(
        &mut self,
        attribute: &Attribute,
        attribute_index: usize,
    ) {
        // Assign the dynamic index to the attribute
        attribute.set_dyn_idx(self.body.attr_cursors.len());

        // And then save the current cursor as the corresponding cursor
        self.body
            .attr_cursors
            .push((self.current_cursor.clone(), attribute_index));
    }
}

impl TemplateBody {
    /// Cascade down path information into the children of this template
    ///
    /// This provides the necessary path and index information for the children of this template
    /// so that they can render out their dynamic nodes correctly. Also does plumbing for things like
    /// hotreloaded literals which need to be tracked on a per-template basis.
    ///
    /// This can only operate with knowledge of this template, not the surrounding callbody. Things like
    /// wiring of ifmt literals need to be done at the callbody level since those final IDs need to
    /// be unique to the entire app.
    pub(crate) fn assign_paths_inner(&mut self, nodes: &[BodyNode]) {
        let mut visitor = DynIdVisitor::new(self);
        visitor.visit_children(nodes);
    }
}
