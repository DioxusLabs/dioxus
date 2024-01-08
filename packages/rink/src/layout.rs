use std::sync::{Arc, Mutex};

use dioxus_native_core::exports::shipyard::Component;
use dioxus_native_core::layout_attributes::{
    apply_layout_attributes_cfg, BorderWidths, LayoutConfigeration,
};
use dioxus_native_core::node::OwnedAttributeView;
use dioxus_native_core::node_ref::{AttributeMaskBuilder, NodeMaskBuilder, NodeView};
use dioxus_native_core::prelude::*;
use dioxus_native_core_macro::partial_derive_state;
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
}
impl<T> Default for PossiblyUninitalized<T> {
    fn default() -> Self {
        Self::Uninitalized
    }
}

#[derive(Clone, PartialEq, Default, Debug, Component)]
pub(crate) struct TaffyLayout {
    pub style: Style,
    pub node: PossiblyUninitalized<Node>,
}

#[partial_derive_state]
impl State for TaffyLayout {
    type ChildDependencies = (Self,);
    type ParentDependencies = ();
    type NodeDependencies = ();

    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new()
        .with_attrs(AttributeMaskBuilder::Some(SORTED_LAYOUT_ATTRS))
        .with_text();

    // The layout state should be effected by the shadow dom
    const TRAVERSE_SHADOW_DOM: bool = true;

    fn update<'a>(
        &mut self,
        node_view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        _: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        ctx: &SendAnyMap,
    ) -> bool {
        let mut changed = false;
        let taffy: &Arc<Mutex<Taffy>> = ctx.get().unwrap();
        let mut taffy = taffy.lock().expect("poisoned taffy");
        let mut style = Style::default();
        if let Some(text) = node_view.text() {
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
                    taffy.set_style(n, style.clone()).unwrap();
                }
            } else {
                self.node =
                    PossiblyUninitalized::Initialized(taffy.new_leaf(style.clone()).unwrap());
                changed = true;
            }
        } else {
            // gather up all the styles from the attribute list
            if let Some(attributes) = node_view.attributes() {
                for OwnedAttributeView {
                    attribute, value, ..
                } in attributes
                {
                    if value.as_custom().is_none() {
                        apply_layout_attributes_cfg(
                            &attribute.name,
                            &value.to_string(),
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

            fn scale_dimension(d: Dimension) -> Dimension {
                match d {
                    Dimension::Points(p) => Dimension::Points(unit_to_layout_space(p)),
                    Dimension::Percent(p) => Dimension::Percent(p),
                    Dimension::Auto => Dimension::Auto,
                }
            }

            fn scale_length_percentage_auto(d: LengthPercentageAuto) -> LengthPercentageAuto {
                match d {
                    LengthPercentageAuto::Points(p) => {
                        LengthPercentageAuto::Points(unit_to_layout_space(p))
                    }
                    LengthPercentageAuto::Percent(p) => LengthPercentageAuto::Percent(p),
                    LengthPercentageAuto::Auto => LengthPercentageAuto::Auto,
                }
            }

            fn scale_length_percentage(d: LengthPercentage) -> LengthPercentage {
                match d {
                    LengthPercentage::Points(p) => {
                        LengthPercentage::Points(unit_to_layout_space(p))
                    }
                    LengthPercentage::Percent(p) => LengthPercentage::Percent(p),
                }
            }

            let scaled_style = Style {
                inset: Rect {
                    left: scale_length_percentage_auto(style.inset.left),
                    right: scale_length_percentage_auto(style.inset.right),
                    top: scale_length_percentage_auto(style.inset.top),
                    bottom: scale_length_percentage_auto(style.inset.bottom),
                },
                margin: Rect {
                    left: scale_length_percentage_auto(style.margin.left),
                    right: scale_length_percentage_auto(style.margin.right),
                    top: scale_length_percentage_auto(style.margin.top),
                    bottom: scale_length_percentage_auto(style.margin.bottom),
                },
                padding: Rect {
                    left: scale_length_percentage(style.padding.left),
                    right: scale_length_percentage(style.padding.right),
                    top: scale_length_percentage(style.padding.top),
                    bottom: scale_length_percentage(style.padding.bottom),
                },
                border: Rect {
                    left: scale_length_percentage(style.border.left),
                    right: scale_length_percentage(style.border.right),
                    top: scale_length_percentage(style.border.top),
                    bottom: scale_length_percentage(style.border.bottom),
                },
                gap: Size {
                    width: scale_length_percentage(style.gap.width),
                    height: scale_length_percentage(style.gap.height),
                },
                flex_basis: scale_dimension(style.flex_basis),
                size: Size {
                    width: scale_dimension(style.size.width),
                    height: scale_dimension(style.size.height),
                },
                min_size: Size {
                    width: scale_dimension(style.min_size.width),
                    height: scale_dimension(style.min_size.height),
                },
                max_size: Size {
                    width: scale_dimension(style.max_size.width),
                    height: scale_dimension(style.max_size.height),
                },
                ..style.clone()
            };

            if let PossiblyUninitalized::Initialized(n) = self.node {
                if self.style != style {
                    taffy.set_style(n, scaled_style).unwrap();
                }
                if taffy.children(n).unwrap() != child_layout {
                    taffy.set_children(n, &child_layout).unwrap();
                }
            } else {
                self.node = PossiblyUninitalized::Initialized(
                    taffy
                        .new_with_children(scaled_style, &child_layout)
                        .unwrap(),
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

    fn create<'a>(
        node_view: NodeView<()>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> Self {
        let mut myself = Self::default();
        myself.update(node_view, node, parent, children, context);
        myself
    }
}

// these are the attributes in layout_attiributes in native-core
const SORTED_LAYOUT_ATTRS: &[&str] = &[
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
    "z-index",
];
