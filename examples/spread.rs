//! This example demonstrates how to use the spread operator to pass attributes to child components.
//!
//! This lets components like the `Link` allow the user to extend the attributes of the underlying `a` tag.
//! These attributes are bundled into a `Vec<Attribute>` which can be spread into the child component using the `..` operator.

use dioxus::prelude::*;

fn main() {
    let dom = VirtualDom::prebuilt(app);
    let html = dioxus_ssr::render(&dom);

    println!("{}", html);
}

// fn app() -> Element {
//     dioxus_core::Element::Ok({
//         #[doc(hidden)]
//         static ___TEMPLATE: dioxus_core::Template = dioxus_core::Template {
//             name: "examples/spread.rs:16:5:0",
//             roots: &[dioxus_core::TemplateNode::Dynamic { id: 0usize }],
//             node_paths: &[&[0u8]],
//             attr_paths: &[],
//         };
//         {
//             #[allow(clippy::let_and_return)]
//             let __vnodes =
//                 dioxus_core::VNode::new(
//                     None,
//                     ___TEMPLATE,
//                     Box::new([dioxus_core::DynamicNode::Component({
//                         use dioxus_core::prelude::Properties;
//                         ({
//                             fc_to_builder(SpreadableComponent)
//                                 .width({
//                                     #[cfg(debug_assertions)]
//                                     {
//                                         static __SIGNAL: GlobalSignal<&'static str> =
//                                             GlobalSignal::with_key(|| "10px", {
//                                                 "examples/spread.rs:16:5:0"
//                                             });
//                                         _ = format_args!("10px");
//                                         __SIGNAL.with(|s| s.clone() as &'static str)
//                                     }
//                                 })
//                                 .extra_data({
//                                     #[cfg(debug_assertions)]
//                                     {
//                                         static __SIGNAL: GlobalSignal<FmtedSegments> =
//                                             GlobalSignal::with_key(
//                                                 || {
//                                                     FmtedSegments::new(<[_]>::into_vec(Box::new([
//                                                         FmtSegment::Literal { value: "hello" },
//                                                         FmtSegment::Dynamic { id: 0usize },
//                                                     ])))
//                                                 },
//                                                 { "examples/spread.rs:16:5:1" },
//                                             );
//                                         _ = format_args!("hello{0}", 1);
//                                         __SIGNAL.with(|s| {
//                                             s.render_with(<[_]>::into_vec(Box::new([
//                                                 format_args!("{0}", 1).to_string(),
//                                             ])))
//                                         })
//                                     }
//                                 })
//                                 .extra_data2({
//                                     #[cfg(debug_assertions)]
//                                     {
//                                         static __SIGNAL: GlobalSignal<FmtedSegments> =
//                                             GlobalSignal::with_key(
//                                                 || {
//                                                     FmtedSegments::new(<[_]>::into_vec(Box::new([
//                                                         FmtSegment::Literal { value: "hello" },
//                                                         FmtSegment::Dynamic { id: 0usize },
//                                                     ])))
//                                                 },
//                                                 { "examples/spread.rs:16:5:2" },
//                                             );
//                                         _ = format_args!("hello{0}", 2);
//                                         __SIGNAL.with(|s| {
//                                             s.render_with(<[_]>::into_vec(Box::new([
//                                                 format_args!("{0}", 2).to_string(),
//                                             ])))
//                                         })
//                                     }
//                                 })
//                                 .height({
//                                     #[cfg(debug_assertions)]
//                                     {
//                                         static __SIGNAL: GlobalSignal<&'static str> =
//                                             GlobalSignal::with_key(|| "10px", {
//                                                 "examples/spread.rs:16:5:3"
//                                             });
//                                         _ = format_args!("10px");
//                                         __SIGNAL.with(|s| s.clone() as &'static str)
//                                     }
//                                 })
//                                 .left({
//                                     #[cfg(debug_assertions)]
//                                     {
//                                         // static __SIGNAL: GlobalSignal<_> =
//                                         //     GlobalSignal::with_key(|| 1 as _, {
//                                         //         "examples/spread.rs:16:5:4"
//                                         //     });
//                                         // _ = 1;
//                                         // __SIGNAL.with(|s| s.clone())
//                                         GlobalSignal::with_key(|| 1, {
//                                             "examples/spread.rs:16:5:4"
//                                         })
//                                         .with(|s| s.clone())
//                                     }
//                                 })
//                                 .build()
//                         })
//                         .into_vcomponent(SpreadableComponent, "SpreadableComponent")
//                     })]),
//                     Box::new([]),
//                 );
//             __vnodes
//         }
//     })
// }

fn app() -> Element {
    rsx! {
        SpreadableComponent {
            width: "10px",
            extra_data: "hello{1}",
            extra_data2: "hello{2}",
            height: "10px",
            left: 1
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct Props {
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,

    extra_data: String,

    extra_data2: String,
}

#[component]
fn SpreadableComponent(props: Props) -> Element {
    rsx! {
        audio { ..props.attributes, "1: {props.extra_data}\n2: {props.extra_data2}" }
    }
}
