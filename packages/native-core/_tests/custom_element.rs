use dioxus::prelude::*;
use dioxus_native_core::{custom_element::CustomElement, prelude::*};
use dioxus_native_core_macro::partial_derive_state;
use shipyard::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Component)]
pub struct ColorState {
    color: usize,
}

#[partial_derive_state]
impl State for ColorState {
    type ParentDependencies = (Self,);
    type ChildDependencies = ();
    type NodeDependencies = ();

    // The color state should not be effected by the shadow dom
    const TRAVERSE_SHADOW_DOM: bool = false;

    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new()
        .with_attrs(AttributeMaskBuilder::Some(&["color"]))
        .with_element();

    fn update<'a>(
        &mut self,
        view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: &SendAnyMap,
    ) -> bool {
        if let Some(size) = view
            .attributes()
            .into_iter()
            .flatten()
            .find(|attr| attr.attribute.name == "color")
        {
            self.color = size
                .value
                .as_float()
                .or_else(|| size.value.as_int().map(|i| i as f64))
                .or_else(|| size.value.as_text().and_then(|i| i.parse().ok()))
                .unwrap_or(0.0) as usize;
        } else if let Some((parent,)) = parent {
            *self = *parent;
        }
        true
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Component)]
pub struct LayoutState {
    size: usize,
}

#[partial_derive_state]
impl State for LayoutState {
    type ParentDependencies = (Self,);
    type ChildDependencies = ();
    type NodeDependencies = ();

    // The layout state should be effected by the shadow dom
    const TRAVERSE_SHADOW_DOM: bool = true;

    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new()
        .with_attrs(AttributeMaskBuilder::Some(&["size"]))
        .with_element();

    fn update<'a>(
        &mut self,
        view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: &SendAnyMap,
    ) -> bool {
        if let Some(size) = view
            .attributes()
            .into_iter()
            .flatten()
            .find(|attr| attr.attribute.name == "size")
        {
            self.size = size
                .value
                .as_float()
                .or_else(|| size.value.as_int().map(|i| i as f64))
                .or_else(|| size.value.as_text().and_then(|i| i.parse().ok()))
                .unwrap_or(0.0) as usize;
        } else if let Some((parent,)) = parent {
            if parent.size > 0 {
                self.size = parent.size - 1;
            }
        }
        true
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

mod dioxus_elements {
    macro_rules! builder_constructors {
        (
            $(
                $(#[$attr:meta])*
                $name:ident {
                    $(
                        $(#[$attr_method:meta])*
                        $fil:ident: $vil:ident,
                    )*
                };
            )*
        ) => {
            $(
                #[allow(non_camel_case_types)]
                $(#[$attr])*
                pub struct $name;

                #[allow(non_upper_case_globals, unused)]
                impl $name {
                    pub const TAG_NAME: &'static str = stringify!($name);
                    pub const NAME_SPACE: Option<&'static str> = None;

                    $(
                        pub const $fil: (&'static str, Option<&'static str>, bool) = (stringify!($fil), None, false);
                    )*
                }

                impl GlobalAttributes for $name {}
            )*
        }
    }

    pub trait GlobalAttributes {}

    pub trait SvgAttributes {}

    builder_constructors! {
        customelementslot {
            size: attr,
            color: attr,
        };
        customelementnoslot {
            size: attr,
            color: attr,
        };
        testing132 {
            color: attr,
        };
    }
}

struct CustomElementWithSlot {
    root: NodeId,
    slot: NodeId,
}

impl CustomElement for CustomElementWithSlot {
    const NAME: &'static str = "customelementslot";

    fn create(mut node: NodeMut<()>) -> Self {
        let dom = node.real_dom_mut();
        let child = dom.create_node(ElementNode {
            tag: "div".into(),
            namespace: None,
            attributes: Default::default(),
            listeners: Default::default(),
        });
        let slot_id = child.id();
        let mut root = dom.create_node(ElementNode {
            tag: "div".into(),
            namespace: None,
            attributes: Default::default(),
            listeners: Default::default(),
        });
        root.add_child(slot_id);

        Self {
            root: root.id(),
            slot: slot_id,
        }
    }

    fn slot(&self) -> Option<NodeId> {
        Some(self.slot)
    }

    fn roots(&self) -> Vec<NodeId> {
        vec![self.root]
    }

    fn attributes_changed(
        &mut self,
        node: NodeMut<()>,
        attributes: &dioxus_native_core::node_ref::AttributeMask,
    ) {
        println!("attributes_changed");
        println!("{:?}", attributes);
        println!("{:?}: {:#?}", node.id(), &*node.node_type());
    }
}

struct CustomElementWithNoSlot {
    root: NodeId,
}

impl CustomElement for CustomElementWithNoSlot {
    const NAME: &'static str = "customelementnoslot";

    fn create(mut node: NodeMut<()>) -> Self {
        let dom = node.real_dom_mut();
        let root = dom.create_node(ElementNode {
            tag: "div".into(),
            namespace: None,
            attributes: Default::default(),
            listeners: Default::default(),
        });
        Self { root: root.id() }
    }

    fn roots(&self) -> Vec<NodeId> {
        vec![self.root]
    }

    fn attributes_changed(
        &mut self,
        node: NodeMut<()>,
        attributes: &dioxus_native_core::node_ref::AttributeMask,
    ) {
        println!("attributes_changed");
        println!("{:?}", attributes);
        println!("{:?}: {:#?}", node.id(), &*node.node_type());
    }
}

#[test]
fn custom_elements_work() {
    fn app() -> Element {
        let count = use_signal(|| 0);

        use_future(|count| async move {
            count.with_mut(|count| *count += 1);
        });

        rsx! {
            customelementslot {
                size: "{count}",
                color: "1",
                customelementslot {
                    testing132 {}
                }
            }
        }
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut rdom = RealDom::new([LayoutState::to_type_erased(), ColorState::to_type_erased()]);
        rdom.register_custom_element::<CustomElementWithSlot>();
        let mut dioxus_state = DioxusState::create(&mut rdom);
        let mut dom = VirtualDom::new(app);

        let mutations = dom.rebuild();
        dioxus_state.apply_mutations(&mut rdom, mutations);

        let ctx = SendAnyMap::new();
        rdom.update_state(ctx);

        for i in 0..10usize {
            dom.wait_for_work().await;

            let mutations = dom.render_immediate();
            dioxus_state.apply_mutations(&mut rdom, mutations);

            let ctx = SendAnyMap::new();
            rdom.update_state(ctx);

            // render...
            rdom.traverse_depth_first_advanced(true, |node| {
                let node_type = &*node.node_type();
                let height = node.height() as usize;
                let indent = " ".repeat(height);
                let color = *node.get::<ColorState>().unwrap();
                let size = *node.get::<LayoutState>().unwrap();
                let id = node.id();
                println!("{indent}{id:?} {color:?} {size:?} {node_type:?}");
                if let NodeType::Element(el) = node_type {
                    match el.tag.as_str() {
                        // the color should bubble up from customelementslot
                        "testing132" | "customelementslot" => {
                            assert_eq!(color.color, 1);
                        }
                        // the color of the light dom should not effect the color of the shadow dom, so the color of divs in the shadow dom should be 0
                        "div" => {
                            assert_eq!(color.color, 0);
                        }
                        _ => {}
                    }
                    if el.tag != "Root" {
                        assert_eq!(size.size, (i + 2).saturating_sub(height));
                    }
                }
            });
        }
    });
}

#[test]
#[should_panic]
fn slotless_custom_element_cant_have_children() {
    fn app() -> Element {
        rsx! {
            customelementnoslot {
                testing132 {}
            }
        }
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut rdom = RealDom::new([LayoutState::to_type_erased(), ColorState::to_type_erased()]);
        rdom.register_custom_element::<CustomElementWithNoSlot>();
        let mut dioxus_state = DioxusState::create(&mut rdom);
        let mut dom = VirtualDom::new(app);

        let mutations = dom.rebuild();
        dioxus_state.apply_mutations(&mut rdom, mutations);

        let ctx = SendAnyMap::new();
        rdom.update_state(ctx);
    });
}

#[test]
fn slotless_custom_element() {
    fn app() -> Element {
        rsx! {
            customelementnoslot {
            }
        }
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut rdom = RealDom::new([LayoutState::to_type_erased(), ColorState::to_type_erased()]);
        rdom.register_custom_element::<CustomElementWithNoSlot>();
        let mut dioxus_state = DioxusState::create(&mut rdom);
        let mut dom = VirtualDom::new(app);

        let mutations = dom.rebuild();
        dioxus_state.apply_mutations(&mut rdom, mutations);

        let ctx = SendAnyMap::new();
        rdom.update_state(ctx);
    });
}
