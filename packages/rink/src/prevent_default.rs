use dioxus_native_core::prelude::*;
use dioxus_native_core_macro::partial_derive_state;
use shipyard::Component;

#[derive(PartialEq, Debug, Clone, Copy, Component, Default)]
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

#[partial_derive_state]
impl State for PreventDefault {
    type ParentDependencies = ();
    type ChildDependencies = ();
    type NodeDependencies = ();

    const NODE_MASK: dioxus_native_core::node_ref::NodeMaskBuilder<'static> =
        dioxus_native_core::node_ref::NodeMaskBuilder::new()
            .with_attrs(dioxus_native_core::node_ref::AttributeMaskBuilder::Some(&[
                "dioxus-prevent-default",
            ]))
            .with_listeners();

    fn update<'a>(
        &mut self,
        node_view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        _: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: &SendAnyMap,
    ) -> bool {
        let new = match node_view.attributes().and_then(|mut attrs| {
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
