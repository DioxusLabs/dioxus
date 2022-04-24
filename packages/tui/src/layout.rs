use dioxus_core::*;
use std::collections::HashMap;

use crate::{
    attributes::{apply_attributes, StyleModifer},
    style::RinkStyle,
    TuiModifier, TuiNode,
};

/*
The layout system uses the lineheight as one point.

stretch uses fractional points, so we can rasterize if we need too, but not with characters
this means anything thats "1px" is 1 lineheight. Unfortunately, text cannot be smaller or bigger
*/
pub fn collect_layout<'a>(
    layout: &mut stretch2::Stretch,
    nodes: &mut HashMap<ElementId, TuiNode<'a>>,
    vdom: &'a VirtualDom,
    node: &'a VNode<'a>,
) {
    use stretch2::prelude::*;

    match node {
        VNode::Text(t) => {
            let id = t.id.get().unwrap();
            let char_len = t.text.chars().count();

            let style = Style {
                size: Size {
                    // characters are 1 point tall
                    height: Dimension::Points(1.0),

                    // text is as long as it is declared
                    width: Dimension::Points(char_len as f32),
                },
                ..Default::default()
            };

            nodes.insert(
                id,
                TuiNode {
                    node,
                    block_style: RinkStyle::default(),
                    tui_modifier: TuiModifier::default(),
                    layout: layout.new_node(style, &[]).unwrap(),
                },
            );
        }
        VNode::Element(el) => {
            // gather up all the styles from the attribute list
            let mut modifier = StyleModifer {
                style: Style::default(),
                tui_style: RinkStyle::default(),
                tui_modifier: TuiModifier::default(),
            };

            // handle text modifier elements
            if el.namespace.is_none() {
                match el.tag {
                    "b" => apply_attributes("font-weight", "bold", &mut modifier),
                    "strong" => apply_attributes("font-weight", "bold", &mut modifier),
                    "u" => apply_attributes("text-decoration", "underline", &mut modifier),
                    "ins" => apply_attributes("text-decoration", "underline", &mut modifier),
                    "del" => apply_attributes("text-decoration", "line-through", &mut modifier),
                    "i" => apply_attributes("font-style", "italic", &mut modifier),
                    "em" => apply_attributes("font-style", "italic", &mut modifier),
                    "mark" => apply_attributes(
                        "background-color",
                        "rgba(241, 231, 64, 50%)",
                        &mut modifier,
                    ),
                    _ => (),
                }
            }

            for &Attribute { name, value, .. } in el.attributes {
                apply_attributes(name, value, &mut modifier);
            }

            // Layout the children
            for child in el.children {
                collect_layout(layout, nodes, vdom, child);
            }

            // Set all direct nodes as our children
            let mut child_layout = vec![];
            for el in el.children {
                let ite = ElementIdIterator::new(vdom, el);
                for node in ite {
                    match node {
                        VNode::Element(_) | VNode::Text(_) => {
                            //
                            child_layout.push(nodes[&node.mounted_id()].layout)
                        }
                        VNode::Placeholder(_) => {}
                        VNode::Fragment(_) => todo!(),
                        VNode::Component(_) => todo!(),
                    }

                    // child_layout.push(nodes[&node.mounted_id()].layout)
                }
            }

            nodes.insert(
                node.mounted_id(),
                TuiNode {
                    node,
                    block_style: modifier.tui_style,
                    tui_modifier: modifier.tui_modifier,
                    layout: layout.new_node(modifier.style, &child_layout).unwrap(),
                },
            );
        }
        VNode::Fragment(el) => {
            //
            for child in el.children {
                collect_layout(layout, nodes, vdom, child);
            }
        }
        VNode::Component(sc) => {
            //
            let scope = vdom.get_scope(sc.scope.get().unwrap()).unwrap();
            let root = scope.root_node();
            collect_layout(layout, nodes, vdom, root);
        }
        VNode::Placeholder(_) => {
            //
        }
    };
}
