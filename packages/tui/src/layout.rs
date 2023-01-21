use std::sync::{Arc, Mutex};

use dioxus_native_core::layout_attributes::{
    apply_layout_attributes_cfg, BorderWidths, LayoutConfigeration,
};
use dioxus_native_core::node::OwnedAttributeView;
use dioxus_native_core::node_ref::{AttributeMask, NodeMask, NodeView};
use dioxus_native_core::state::ChildDepState;
use dioxus_native_core_macro::sorted_str_slice;
use taffy::prelude::*;

use crate::{screen_to_layout_space, unit_to_layout_space};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PossiblyUninitalized<T> {
    Uninitalized,
    Initialized(T),
}
impl<T> PossiblyUninitalized<T> {
    pub fn unwrap(self) -> T {
        match self {
            Self::Initialized(i) => i,
            _ => panic!("uninitalized"),
        }
    }
    pub fn ok(self) -> Option<T> {
        match self {
            Self::Initialized(i) => Some(i),
            _ => None,
        }
    }
}
impl<T> Default for PossiblyUninitalized<T> {
    fn default() -> Self {
        Self::Uninitalized
    }
}

#[derive(Clone, PartialEq, Default, Debug)]
pub(crate) struct TaffyLayout {
    pub style: Style,
    pub node: PossiblyUninitalized<Node>,
}

impl ChildDepState for TaffyLayout {
    type Ctx = Arc<Mutex<Taffy>>;
    type DepState = (Self,);
    // use tag to force this to be called when a node is built
    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(SORTED_LAYOUT_ATTRS))
            .with_text()
            .with_tag();

    /// Setup the layout
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        children: impl Iterator<Item = (&'a Self,)>,
        ctx: &Self::Ctx,
    ) -> bool
    where
        Self::DepState: 'a,
    {
        let mut changed = false;
        let mut taffy = ctx.lock().expect("poisoned taffy");
        let mut style = Style::default();
        if let Some(text) = node.text() {
            let char_len = text.chars().count();

            style = Style {
                size: Size {
                    // characters are 1 point tall
                    height: Dimension::Points(screen_to_layout_space(1)),

                    // text is as long as it is declared
                    width: Dimension::Points(screen_to_layout_space(char_len as u16)),
                },
                ..Default::default()
            };
            if let PossiblyUninitalized::Initialized(n) = self.node {
                if self.style != style {
                    taffy.set_style(n, style).unwrap();
                }
            } else {
                self.node = PossiblyUninitalized::Initialized(taffy.new_leaf(style).unwrap());
                changed = true;
            }
        } else {
            // gather up all the styles from the attribute list
            if let Some(attributes) = node.attributes() {
                for OwnedAttributeView {
                    attribute, value, ..
                } in attributes
                {
                    assert!(SORTED_LAYOUT_ATTRS
                        .binary_search(&attribute.name.as_ref())
                        .is_ok());
                    if let Some(text) = value.as_text() {
                        apply_layout_attributes_cfg(
                            &attribute.name,
                            text,
                            &mut style,
                            &LayoutConfigeration {
                                border_widths: BorderWidths {
                                    thin: 1.0,
                                    medium: 1.0,
                                    thick: 1.0,
                                },
                            },
                        );
                    }
                }
            }

            // Set all direct nodes as our children
            let mut child_layout = vec![];
            for (l,) in children {
                child_layout.push(l.node.unwrap());
            }

            fn scale_dimention(d: Dimension) -> Dimension {
                match d {
                    Dimension::Points(p) => Dimension::Points(unit_to_layout_space(p)),
                    Dimension::Percent(p) => Dimension::Percent(p),
                    Dimension::Auto => Dimension::Auto,
                    Dimension::Undefined => Dimension::Undefined,
                }
            }
            let style = Style {
                position: Rect {
                    left: scale_dimention(style.position.left),
                    right: scale_dimention(style.position.right),
                    top: scale_dimention(style.position.top),
                    bottom: scale_dimention(style.position.bottom),
                },
                margin: Rect {
                    left: scale_dimention(style.margin.left),
                    right: scale_dimention(style.margin.right),
                    top: scale_dimention(style.margin.top),
                    bottom: scale_dimention(style.margin.bottom),
                },
                padding: Rect {
                    left: scale_dimention(style.padding.left),
                    right: scale_dimention(style.padding.right),
                    top: scale_dimention(style.padding.top),
                    bottom: scale_dimention(style.padding.bottom),
                },
                border: Rect {
                    left: scale_dimention(style.border.left),
                    right: scale_dimention(style.border.right),
                    top: scale_dimention(style.border.top),
                    bottom: scale_dimention(style.border.bottom),
                },
                gap: Size {
                    width: scale_dimention(style.gap.width),
                    height: scale_dimention(style.gap.height),
                },
                flex_basis: scale_dimention(style.flex_basis),
                size: Size {
                    width: scale_dimention(style.size.width),
                    height: scale_dimention(style.size.height),
                },
                min_size: Size {
                    width: scale_dimention(style.min_size.width),
                    height: scale_dimention(style.min_size.height),
                },
                max_size: Size {
                    width: scale_dimention(style.max_size.width),
                    height: scale_dimention(style.max_size.height),
                },
                ..style
            };

            if let PossiblyUninitalized::Initialized(n) = self.node {
                if self.style != style {
                    taffy.set_style(n, style).unwrap();
                }
                if taffy.children(n).unwrap() != child_layout {
                    taffy.set_children(n, &child_layout).unwrap();
                }
            } else {
                self.node = PossiblyUninitalized::Initialized(
                    taffy.new_with_children(style, &child_layout).unwrap(),
                );
                changed = true;
            }
        }
        if self.style != style {
            changed = true;
            self.style = style;
        }
        changed
    }
}

// these are the attributes in layout_attiributes in native-core
const SORTED_LAYOUT_ATTRS: &[&str] = &sorted_str_slice!([
    "align-content",
    "align-items",
    "align-self",
    "animation",
    "animation-delay",
    "animation-direction",
    "animation-duration",
    "animation-fill-mode",
    "animation-iteration-count",
    "animation-name",
    "animation-play-state",
    "animation-timing-function",
    "backface-visibility",
    "border",
    "border-bottom",
    "border-bottom-color",
    "border-bottom-left-radius",
    "border-bottom-right-radius",
    "border-bottom-style",
    "border-bottom-width",
    "border-collapse",
    "border-color",
    "border-image",
    "border-image-outset",
    "border-image-repeat",
    "border-image-slice",
    "border-image-source",
    "border-image-width",
    "border-left",
    "border-left-color",
    "border-left-style",
    "border-left-width",
    "border-radius",
    "border-right",
    "border-right-color",
    "border-right-style",
    "border-right-width",
    "border-spacing",
    "border-style",
    "border-top",
    "border-top-color",
    "border-top-left-radius",
    "border-top-right-radius",
    "border-top-style",
    "border-top-width",
    "border-width",
    "bottom",
    "box-shadow",
    "box-sizing",
    "caption-side",
    "clear",
    "clip",
    "column-count",
    "column-fill",
    "column-gap",
    "column-rule",
    "column-rule-color",
    "column-rule-style",
    "column-rule-width",
    "column-span",
    "column-width",
    "columns",
    "content",
    "counter-increment",
    "counter-reset",
    "cursor",
    "direction",
    "ltr",
    "rtl",
    "display",
    "empty-cells",
    "flex",
    "flex-basis",
    "flex-direction",
    "flex-flow",
    "flex-grow",
    "flex-shrink",
    "flex-wrap",
    "float",
    "height",
    "justify-content",
    "flex-start",
    "flex-end",
    "center",
    "space-between",
    "space-around",
    "space-evenly",
    "left",
    "letter-spacing",
    "line-height",
    "list-style",
    "list-style-image",
    "list-style-position",
    "list-style-type",
    "margin",
    "margin-bottom",
    "margin-left",
    "margin-right",
    "margin-top",
    "max-height",
    "max-width",
    "min-height",
    "min-width",
    "opacity",
    "order",
    "outline",
    "outline-color",
    "outline-offset",
    "outline-style",
    "outline-width",
    "overflow",
    "overflow-x",
    "overflow-y",
    "padding",
    "padding-bottom",
    "padding-left",
    "padding-right",
    "padding-top",
    "page-break-after",
    "page-break-before",
    "page-break-inside",
    "perspective",
    "perspective-origin",
    "position",
    "static",
    "relative",
    "fixed",
    "absolute",
    "sticky",
    "pointer-events",
    "quotes",
    "resize",
    "right",
    "tab-size",
    "table-layout",
    "top",
    "transform",
    "transform-origin",
    "transform-style",
    "transition",
    "transition-delay",
    "transition-duration",
    "transition-property",
    "transition-timing-function",
    "vertical-align",
    "visibility",
    "white-space",
    "width",
    "word-break",
    "word-spacing",
    "word-wrap",
    "z-index"
]);
