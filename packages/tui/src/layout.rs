use std::cell::RefCell;
use std::rc::Rc;

use dioxus_core::*;
use dioxus_native_core::layout_attributes::apply_layout_attributes;
use dioxus_native_core::state::{AttributeMask, ChildDepState, NodeMask, NodeView};
use stretch2::prelude::*;

#[derive(Clone, PartialEq, Default, Debug)]
pub struct StretchLayout {
    pub style: Style,
    pub node: Option<Node>,
}

impl ChildDepState for StretchLayout {
    type Ctx = Rc<RefCell<Stretch>>;
    type DepState = Self;
    // todo: update mask
    const NODE_MASK: NodeMask = NodeMask::new(AttributeMask::All, false, false, true);

    /// Setup the layout
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        children: impl Iterator<Item = &'a Self::DepState>,
        ctx: &Self::Ctx,
    ) -> bool
    where
        Self::DepState: 'a,
    {
        let mut changed = false;
        let mut stretch = ctx.borrow_mut();
        let mut style = Style::default();
        if let Some(text) = node.text() {
            let char_len = text.chars().count();

            style = Style {
                size: Size {
                    // characters are 1 point tall
                    height: Dimension::Points(1.0),

                    // text is as long as it is declared
                    width: Dimension::Points(char_len as f32),
                },
                ..Default::default()
            };
            if let Some(n) = self.node {
                if self.style != style {
                    stretch.set_style(n, style).unwrap();
                }
            } else {
                self.node = Some(stretch.new_node(style, &[]).unwrap());
            }
        } else {
            // gather up all the styles from the attribute list
            for &Attribute { name, value, .. } in node.attributes() {
                apply_layout_attributes(name, value, &mut style);
            }

            // the root node fills the entire area
            if node.id() == ElementId(0) {
                apply_layout_attributes("width", "100%", &mut style);
                apply_layout_attributes("height", "100%", &mut style);
            }

            // Set all direct nodes as our children
            let mut child_layout = vec![];
            for l in children {
                child_layout.push(l.node.unwrap());
            }

            if let Some(n) = self.node {
                if self.style != style {
                    stretch.set_style(n, style).unwrap();
                }
            } else {
                self.node = Some(stretch.new_node(style, &[]).unwrap());
            }
            if let Some(n) = self.node {
                if self.style != style {
                    stretch.set_style(n, style).unwrap();
                }
                if stretch.children(n).unwrap() != child_layout {
                    stretch.set_children(n, &child_layout).unwrap();
                }
            } else {
                self.node = Some(stretch.new_node(style, &[]).unwrap());
            }
        }
        if self.style != style {
            changed = true;
            self.style = style;
        }
        changed
    }
}
