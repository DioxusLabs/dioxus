#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_native_core::exports::shipyard::Component;
use dioxus_native_core::node_ref::*;
use dioxus_native_core::prelude::*;
use dioxus_native_core_macro::partial_derive_state;

struct FontSize(f64);

// All states need to derive Component
#[derive(Default, Debug, Copy, Clone, Component)]
struct Size(f64, f64);

/// Derive some of the boilerplate for the State implementation
#[partial_derive_state]
impl State for Size {
    type ParentDependencies = ();

    // The size of the current node depends on the size of its children
    type ChildDependencies = (Self,);

    type NodeDependencies = ();

    // Size only cares about the width, height, and text parts of the current node
    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new()
        // Get access to the width and height attributes
        .with_attrs(AttributeMaskBuilder::Some(&["width", "height"]))
        // Get access to the text of the node
        .with_text();

    fn update<'a>(
        &mut self,
        node_view: NodeView<()>,
        _node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        _parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> bool {
        let font_size = context.get::<FontSize>().unwrap().0;
        let mut width;
        let mut height;
        if let Some(text) = node_view.text() {
            // if the node has text, use the text to size our object
            width = text.len() as f64 * font_size;
            height = font_size;
        } else {
            // otherwise, the size is the maximum size of the children
            width = children
                .iter()
                .map(|(item,)| item.0)
                .reduce(|accum, item| if accum >= item { accum } else { item })
                .unwrap_or(0.0);

            height = children
                .iter()
                .map(|(item,)| item.1)
                .reduce(|accum, item| if accum >= item { accum } else { item })
                .unwrap_or(0.0);
        }
        // if the node contains a width or height attribute it overrides the other size
        for a in node_view.attributes().into_iter().flatten() {
            match &*a.attribute.name {
                "width" => width = a.value.as_float().unwrap(),
                "height" => height = a.value.as_float().unwrap(),
                // because Size only depends on the width and height, no other attributes will be passed to the member
                _ => panic!(),
            }
        }
        // to determine what other parts of the dom need to be updated we return a boolean that marks if this member changed
        let changed = (width != self.0) || (height != self.1);
        *self = Self(width, height);
        changed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Component)]
struct TextColor {
    r: u8,
    g: u8,
    b: u8,
}

#[partial_derive_state]
impl State for TextColor {
    // TextColor depends on the TextColor part of the parent
    type ParentDependencies = (Self,);

    type ChildDependencies = ();

    type NodeDependencies = ();

    // TextColor only cares about the color attribute of the current node
    const NODE_MASK: NodeMaskBuilder<'static> =
        // Get access to the color attribute
        NodeMaskBuilder::new().with_attrs(AttributeMaskBuilder::Some(&["color"]));

    fn update<'a>(
        &mut self,
        node_view: NodeView<()>,
        _node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _context: &SendAnyMap,
    ) -> bool {
        // TextColor only depends on the color tag, so getting the first tag is equivilent to looking through all tags
        let new = match node_view
            .attributes()
            .and_then(|mut attrs| attrs.next())
            .and_then(|attr| attr.value.as_text())
        {
            // if there is a color tag, translate it
            Some("red") => TextColor { r: 255, g: 0, b: 0 },
            Some("green") => TextColor { r: 0, g: 255, b: 0 },
            Some("blue") => TextColor { r: 0, g: 0, b: 255 },
            Some(color) => panic!("unknown color {color}"),
            // otherwise check if the node has a parent and inherit that color
            None => match parent {
                Some((parent,)) => *parent,
                None => Self::default(),
            },
        };
        // check if the member has changed
        let changed = new != *self;
        *self = new;
        changed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Component)]
struct Border(bool);

#[partial_derive_state]
impl State for Border {
    // TextColor depends on the TextColor part of the parent
    type ParentDependencies = (Self,);

    type ChildDependencies = ();

    type NodeDependencies = ();

    // Border does not depended on any other member in the current node
    const NODE_MASK: NodeMaskBuilder<'static> =
        // Get access to the border attribute
        NodeMaskBuilder::new().with_attrs(AttributeMaskBuilder::Some(&["border"]));

    fn update<'a>(
        &mut self,
        node_view: NodeView<()>,
        _node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        _parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _context: &SendAnyMap,
    ) -> bool {
        // check if the node contians a border attribute
        let new = Self(
            node_view
                .attributes()
                .and_then(|mut attrs| attrs.next().map(|a| a.attribute.name == "border"))
                .is_some(),
        );
        // check if the member has changed
        let changed = new != *self;
        *self = new;
        changed
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fn app(cx: Scope) -> Element {
        let count = use_state(cx, || 0);

        use_future(cx, (count,), |(count,)| async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                count.set(*count + 1);
            }
        });

        cx.render(rsx! {
            div{
                color: "red",
                "{count}",
                Comp {}
            }
        })
    }

    fn Comp(cx: Scope) -> Element {
        cx.render(rsx! {
            div{
                border: "",
                "hello world"
            }
        })
    }

    // create the vdom, the real_dom, and the binding layer between them
    let mut vdom = VirtualDom::new(app);
    let mut rdom: RealDom = RealDom::new([
        Border::to_type_erased(),
        TextColor::to_type_erased(),
        Size::to_type_erased(),
    ]);
    let mut dioxus_intigration_state = DioxusState::create(&mut rdom);

    let mutations = vdom.rebuild();
    // update the structure of the real_dom tree
    dioxus_intigration_state.apply_mutations(&mut rdom, mutations);
    let mut ctx = SendAnyMap::new();
    // set the font size to 3.3
    ctx.insert(FontSize(3.3));
    // update the State for nodes in the real_dom tree
    let _to_rerender = rdom.update_state(ctx);

    // we need to run the vdom in a async runtime
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            loop {
                // wait for the vdom to update
                vdom.wait_for_work().await;

                // get the mutations from the vdom
                let mutations = vdom.render_immediate();

                // update the structure of the real_dom tree
                dioxus_intigration_state.apply_mutations(&mut rdom, mutations);

                // update the state of the real_dom tree
                let mut ctx = SendAnyMap::new();
                // set the font size to 3.3
                ctx.insert(FontSize(3.3));
                let _to_rerender = rdom.update_state(ctx);

                // render...
                rdom.traverse_depth_first(|node| {
                    let indent = " ".repeat(node.height() as usize);
                    let color = *node.get::<TextColor>().unwrap();
                    let size = *node.get::<Size>().unwrap();
                    let border = *node.get::<Border>().unwrap();
                    let id = node.id();
                    let node = node.node_type();
                    let node_type = &*node;
                    println!("{indent}{id:?} {color:?} {size:?} {border:?} {node_type:?}");
                });
            }
        })
}
