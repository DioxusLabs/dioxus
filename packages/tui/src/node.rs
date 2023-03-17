use crate::focus::Focus;
use crate::layout::TaffyLayout;
use crate::style_attributes::StyleModifier;
use dioxus_native_core::{real_dom::RealDom, state::*};
use dioxus_native_core_macro::{sorted_str_slice, State};

pub(crate) type TuiDom = RealDom<NodeState>;
pub(crate) type TuiNode = dioxus_native_core::node::Node<NodeState>;

#[derive(Debug, Clone, State, Default)]
pub(crate) struct NodeState {
    #[child_dep_state(layout, Mutex<Stretch>)]
    pub layout: TaffyLayout,
    #[parent_dep_state(style)]
    pub style: StyleModifier,
    #[node_dep_state()]
    pub prevent_default: PreventDefault,
    #[node_dep_state()]
    pub focus: Focus,
    pub focused: bool,
}

#[derive(PartialEq, Debug, Clone, Default)]
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
    #[default]
    Unknown,
    MouseOver,
    ContextMenu,
    Wheel,
    MouseUp,
}

impl NodeDepState for PreventDefault {
    type DepState = ();
    type Ctx = ();

    const NODE_MASK: dioxus_native_core::node_ref::NodeMask =
        dioxus_native_core::node_ref::NodeMask::new_with_attrs(
            dioxus_native_core::node_ref::AttributeMask::Static(&sorted_str_slice!([
                "dioxus-prevent-default"
            ])),
        )
        .with_listeners();

    fn reduce(
        &mut self,
        node: dioxus_native_core::node_ref::NodeView,
        _sibling: (),
        _ctx: &Self::Ctx,
    ) -> bool {
        let new = match node.attributes().and_then(|mut attrs| {
            attrs
                .find(|a| a.attribute.name == "dioxus-prevent-default")
                .and_then(|a| a.value.as_text())
        }) {
            Some("onfocus") => PreventDefault::Focus,
            Some("onkeypress") => PreventDefault::KeyPress,
            Some("onkeyrelease") => PreventDefault::KeyRelease,
            Some("onkeydown") => PreventDefault::KeyDown,
            Some("onkeyup") => PreventDefault::KeyUp,
            Some("onclick") => PreventDefault::Click,
            Some("onmousedown") => PreventDefault::MouseDown,
            Some("onmouseup") => PreventDefault::MouseUp,
            Some("onmouseenter") => PreventDefault::MouseEnter,
            Some("onmouseover") => PreventDefault::MouseOver,
            Some("onmouseleave") => PreventDefault::MouseLeave,
            Some("onmouseout") => PreventDefault::MouseOut,
            Some("onwheel") => PreventDefault::Wheel,
            Some("oncontextmenu") => PreventDefault::ContextMenu,
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
