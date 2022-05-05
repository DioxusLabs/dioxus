use crate::focus::Focus;
use crate::layout::StretchLayout;
use crate::style_attributes::StyleModifier;
use dioxus_native_core::{real_dom::RealDom, state::*};
use dioxus_native_core_macro::{sorted_str_slice, State};

pub(crate) type Dom = RealDom<NodeState>;
pub(crate) type Node = dioxus_native_core::real_dom::Node<NodeState>;

#[derive(Debug, Clone, State, Default)]
pub(crate) struct NodeState {
    #[child_dep_state(layout, RefCell<Stretch>)]
    pub layout: StretchLayout,
    // depends on attributes, the C component of it's parent and a u8 context
    #[parent_dep_state(style)]
    pub style: StyleModifier,
    #[node_dep_state()]
    pub prevent_default: PreventDefault,
    #[node_dep_state()]
    pub focus: Focus,
    pub focused: bool,
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) enum PreventDefault {
    Focus,
    KeyPress,
    KeyRelease,
    KeyDown,
    KeyUp,
    MouseDown,
    Click,
    MouseEnter,
    MouseLeave,
    MouseOut,
    Unknown,
    MouseOver,
    ContextMenu,
    Wheel,
    MouseUp,
}

impl Default for PreventDefault {
    fn default() -> Self {
        PreventDefault::Unknown
    }
}

impl NodeDepState for PreventDefault {
    type Ctx = ();

    type DepState = ();

    const NODE_MASK: dioxus_native_core::node_ref::NodeMask =
        dioxus_native_core::node_ref::NodeMask::new_with_attrs(
            dioxus_native_core::node_ref::AttributeMask::Static(&sorted_str_slice!([
                "dioxus-prevent-default"
            ])),
        );

    fn reduce(
        &mut self,
        node: dioxus_native_core::node_ref::NodeView,
        _sibling: &Self::DepState,
        _ctx: &Self::Ctx,
    ) -> bool {
        let new = match node
            .attributes()
            .find(|a| a.name == "dioxus-prevent-default")
            .map(|a| a.value)
        {
            Some("focus") => PreventDefault::Focus,
            Some("keypress") => PreventDefault::KeyPress,
            Some("keyrelease") => PreventDefault::KeyRelease,
            Some("keydown") => PreventDefault::KeyDown,
            Some("keyup") => PreventDefault::KeyUp,
            Some("click") => PreventDefault::Click,
            Some("mousedown") => PreventDefault::MouseDown,
            Some("mouseup") => PreventDefault::MouseUp,
            Some("mouseenter") => PreventDefault::MouseEnter,
            Some("mouseover") => PreventDefault::MouseOver,
            Some("mouseleave") => PreventDefault::MouseLeave,
            Some("mouseout") => PreventDefault::MouseOut,
            Some("wheel") => PreventDefault::Wheel,
            Some("contextmenu") => PreventDefault::ContextMenu,
            _ => return false,
        };
        if new == *self {
            false
        } else {
            *self = new;
            true
        }
    }
}
