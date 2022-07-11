use dioxus_core::{
    OwnedTemplateNode, OwnedTemplateValue, TemplateAttribute, TemplateElement, TemplateNodeId,
    TemplateNodeType, TextTemplate, TextTemplateSegment,
};
use syn::Expr;

use crate::{BodyNode, FormattedSegment, FormattedSegmentType, Segment};

#[derive(Default)]
struct TemplateBuilder {
    nodes: Vec<OwnedTemplateNode>,
    dynamic_context: DynamicTemplateContextBuilder,
}

impl TemplateBuilder {
    pub fn from_root(root: BodyNode) -> Self {
        let mut builder = Self::default();

        builder.build_node(root);

        builder
    }

    fn build_node(&mut self, node: BodyNode) {
        let id = TemplateNodeId(self.nodes.len());
        match node {
            BodyNode::Element(el) => {
                let attributes: Vec<_> = el.attributes.iter().map(|attr| TemplateAttribute {
                    name: todo!(),
                    namespace: todo!(),
                    value: todo!(),
                });
                for child in el.children {
                    self.build_node(child);
                }
                OwnedTemplateNode {
                    id,
                    node_type: TemplateNodeType::Element(TemplateElement {
                        tag: todo!(),
                        namespace: todo!(),
                        attributes,
                        children: todo!(),
                        listeners: todo!(),
                        parent: todo!(),
                        value: todo!(),
                    }),
                }
            }
            BodyNode::Component(comp) => {
                self.nodes.push(OwnedTemplateNode {
                    id,
                    node_type: TemplateNodeType::DynamicNode(
                        self.dynamic_context.add_node(BodyNode::Component(comp)),
                    ),
                });
            }
            BodyNode::Text(txt) => {
                let mut segments = Vec::new();

                for segment in txt.segments {
                    segments.push(match segment {
                        Segment::Literal(lit) => TextTemplateSegment::Static(lit),
                        Segment::Formatted(fmted) => {
                            TextTemplateSegment::Dynamic(self.dynamic_context.add_text(fmted))
                        }
                    })
                }

                self.nodes.push(OwnedTemplateNode {
                    id,
                    node_type: TemplateNodeType::Text(TextTemplate::new(segments)),
                });
            }
            BodyNode::RawExpr(expr) => {
                self.nodes.push(OwnedTemplateNode {
                    id,
                    node_type: TemplateNodeType::DynamicNode(
                        self.dynamic_context.add_node(BodyNode::RawExpr(expr)),
                    ),
                });
            }
        }
    }
}

#[derive(Default)]
struct DynamicTemplateContextBuilder {
    nodes: Vec<BodyNode>,
    text: Vec<FormattedSegment>,
    attributes: Vec<Expr>,
}

impl DynamicTemplateContextBuilder {
    fn add_node(&mut self, node: BodyNode) -> usize {
        let node_id = self.nodes.len();

        self.nodes.push(node);

        node_id
    }

    fn add_text(&mut self, text: FormattedSegment) -> usize {
        let text_id = self.text.len();

        self.text.push(text);

        text_id
    }
}
