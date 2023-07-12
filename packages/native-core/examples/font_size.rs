use dioxus_native_core::exports::shipyard::Component;
use dioxus_native_core::node_ref::*;
use dioxus_native_core::prelude::*;
use dioxus_native_core::real_dom::NodeTypeMut;
use dioxus_native_core_macro::partial_derive_state;

// All states need to derive Component
#[derive(Default, Debug, Copy, Clone, Component)]
struct Size(f64, f64);

/// Derive some of the boilerplate for the State implementation
#[partial_derive_state]
impl State for Size {
    type ParentDependencies = ();

    // The size of the current node depends on the size of its children
    type ChildDependencies = (Self,);

    type NodeDependencies = (FontSize,);

    // Size only cares about the width, height, and text parts of the current node
    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new()
        // Get access to the width and height attributes
        .with_attrs(AttributeMaskBuilder::Some(&["width", "height"]))
        // Get access to the text of the node
        .with_text();

    fn update<'a>(
        &mut self,
        node_view: NodeView<()>,
        (font_size,): <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        _parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: &SendAnyMap,
    ) -> bool {
        let font_size = font_size.size;
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

#[derive(Debug, Clone, Copy, PartialEq, Component)]
struct FontSize {
    size: f64,
}

impl Default for FontSize {
    fn default() -> Self {
        Self { size: 16.0 }
    }
}

#[partial_derive_state]
impl State for FontSize {
    // TextColor depends on the TextColor part of the parent
    type ParentDependencies = (Self,);

    type ChildDependencies = ();

    type NodeDependencies = ();

    // TextColor only cares about the color attribute of the current node
    const NODE_MASK: NodeMaskBuilder<'static> =
        // Get access to the color attribute
        NodeMaskBuilder::new().with_attrs(AttributeMaskBuilder::Some(&["font-size"]));

    fn update<'a>(
        &mut self,
        node_view: NodeView<()>,
        _node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _context: &SendAnyMap,
    ) -> bool {
        let mut new = None;
        for attr in node_view.attributes().into_iter().flatten() {
            if attr.attribute.name == "font-size" {
                new = Some(FontSize {
                    size: attr.value.as_float().unwrap(),
                });
            }
        }
        let new = new.unwrap_or(parent.map(|(p,)| *p).unwrap_or_default());
        // check if the member has changed
        let changed = new != *self;
        *self = new;
        changed
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rdom: RealDom = RealDom::new([FontSize::to_type_erased(), Size::to_type_erased()]);

    let mut count = 0;

    // intial render
    let text_id = rdom.create_node(format!("Count: {count}")).id();
    let mut root = rdom.get_mut(rdom.root_id()).unwrap();
    // set the color to red
    if let NodeTypeMut::Element(mut element) = root.node_type_mut() {
        element.set_attribute(("color", "style"), "red".to_string());
        element.set_attribute(("font-size", "style"), 1.);
    }
    root.add_child(text_id);

    let ctx = SendAnyMap::new();
    // update the State for nodes in the real_dom tree
    let _to_rerender = rdom.update_state(ctx);

    // we need to run the vdom in a async runtime
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            loop {
                // update the count and font size
                count += 1;
                let mut text = rdom.get_mut(text_id).unwrap();
                if let NodeTypeMut::Text(mut text) = text.node_type_mut() {
                    *text = format!("Count: {count}");
                }
                if let NodeTypeMut::Element(mut element) =
                    rdom.get_mut(rdom.root_id()).unwrap().node_type_mut()
                {
                    element.set_attribute(("font-size", "style"), count as f64);
                }

                let ctx = SendAnyMap::new();
                let _to_rerender = rdom.update_state(ctx);

                // render...
                rdom.traverse_depth_first_advanced(true, |node| {
                    let indent = " ".repeat(node.height() as usize);
                    let font_size = *node.get::<FontSize>().unwrap();
                    let size = *node.get::<Size>().unwrap();
                    let id = node.id();
                    println!("{indent}{id:?} {font_size:?} {size:?}");
                });

                // wait 1 second
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        })
}
