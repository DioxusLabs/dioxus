use dioxus::html::input_data::keyboard_types::{Code, Key, Modifiers};
use dioxus::prelude::*;
use dioxus_native_core::exports::shipyard::Component;
use dioxus_native_core::node_ref::*;
use dioxus_native_core::prelude::*;
use dioxus_native_core::utils::cursor::{Cursor, Pos};
use dioxus_native_core_macro::partial_derive_state;

// ANCHOR: state_impl
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
// ANCHOR_END: state_impl

// ANCHOR: rendering
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
                "{count}"
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
// ANCHOR_END: rendering

// ANCHOR: derive_state
// All states must derive Component (https://docs.rs/shipyard/latest/shipyard/derive.Component.html)
// They also must implement Default or provide a custom implementation of create in the State trait
#[derive(Default, Component)]
struct MyState;

/// Derive some of the boilerplate for the State implementation
#[partial_derive_state]
impl State for MyState {
    // The states of the parent nodes this state depends on
    type ParentDependencies = ();

    // The states of the child nodes this state depends on
    type ChildDependencies = (Self,);

    // The states of the current node this state depends on
    type NodeDependencies = ();

    // The parts of the current text, element, or placeholder node in the tree that this state depends on
    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new();

    // How to update the state of the current node based on the state of the parent nodes, child nodes, and the current node
    // Returns true if the node was updated and false if the node was not updated
    fn update<'a>(
        &mut self,
        // The view of the current node limited to the parts this state depends on
        _node_view: NodeView<()>,
        // The state of the current node that this state depends on
        _node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        // The state of the parent nodes that this state depends on
        _parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        // The state of the child nodes that this state depends on
        _children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        // The context of the current node used to pass global state into the tree
        _context: &SendAnyMap,
    ) -> bool {
        todo!()
    }

    // partial_derive_state will generate a default implementation of all the other methods
}
// ANCHOR_END: derive_state

#[allow(unused)]
// ANCHOR: cursor
fn text_editing() {
    let mut cursor = Cursor::default();
    let mut text = String::new();

    // handle keyboard input with a max text length of 10
    cursor.handle_input(
        &Code::ArrowRight,
        &Key::ArrowRight,
        &Modifiers::empty(),
        &mut text,
        10,
    );

    // mannually select text between characters 0-5 on the first line (this could be from dragging with a mouse)
    cursor.start = Pos::new(0, 0);
    cursor.end = Some(Pos::new(5, 0));

    // delete the selected text and move the cursor to the start of the selection
    cursor.delete_selection(&mut text);
}
// ANCHOR_END: cursor
