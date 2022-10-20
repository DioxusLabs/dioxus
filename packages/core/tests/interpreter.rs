use dioxus_core::{prelude::*, TemplateNode, VTemplate, VText};

// #[test]
// fn simple_static() {
//     fn app(cx: Scope) -> Element {
//         static MyTemplate: TemplateDef = TemplateDef {
//             id: "my-template",
//             static_nodes: &[TemplateNode::Element {
//                 attributes: &[],
//                 nodes: &[TemplateNode::StaticText("Hello, world!")],
//                 tag: "div",
//             }],
//             dynamic_nodes: &[],
//         };

//         Some(VNode::Template(NodeFactory::new(&cx).bump().alloc(
//             VTemplate {
//                 def: &MyTemplate,
//                 dynamic_nodes: &[],
//                 rendered_nodes: &[],
//             },
//         )))
//     }

//     let mut dom = VirtualDom::new(app);
//     let edits = dom.rebuild();
//     dbg!(edits);
// }

// #[test]
// fn mixed_dynamic() {
//     fn app(cx: Scope) -> Element {
//         static MyTemplate: TemplateDef = TemplateDef {
//             id: "my-template",
//             static_nodes: &[TemplateNode::Element {
//                 tag: "div",
//                 attributes: &[],
//                 nodes: &[
//                     TemplateNode::StaticText("Hello, world!"),
//                     TemplateNode::DynamicText,
//                 ],
//             }],
//             dynamic_nodes: &[],
//         };

//         let val = cx.use_hook(|| 0);
//         *val += 1;

//         let fact = NodeFactory::new(&cx);

//         Some(VNode::Template(fact.bump().alloc(VTemplateRef {
//             def: &MyTemplate,
//             dynamic_nodes: fact.bump().alloc([fact.text(format_args!("{val}"))]),
//         })))
//     }

//     let mut dom = VirtualDom::new(app);
//     let edits = dom.rebuild();
//     dbg!(edits);

//     let edits = dom.hard_diff(ScopeId(0));
//     dbg!(edits);

//     let edits = dom.hard_diff(ScopeId(0));
//     dbg!(edits);

//     let edits = dom.hard_diff(ScopeId(0));
//     dbg!(edits);
// }

// #[test]
// fn mixes() {
//     fn app(cx: Scope) -> Element {
//         static MyTemplate: TemplateDef = TemplateDef {
//             id: "my-template",
//             static_nodes: &[TemplateNode::Element {
//                 tag: "div",
//                 attributes: &[],
//                 nodes: &[
//                     TemplateNode::StaticText("Hello, world!"),
//                     TemplateNode::DynamicText,
//                 ],
//             }],
//             dynamic_nodes: &[],
//         };

//         let val = cx.use_hook(|| 1);
//         *val += 1;

//         let fact = NodeFactory::new(&cx);

//         if *val % 2 == 0 {
//             Some(VNode::Template(fact.bump().alloc(VTemplateRef {
//                 def: &MyTemplate,
//                 dynamic_nodes: fact.bump().alloc([fact.text(format_args!("{val}"))]),
//             })))
//         } else {
//             Some(fact.text(format_args!("Hello, world! {val}")))
//         }
//     }

//     let mut dom = VirtualDom::new(app);
//     let edits = dom.rebuild();
//     dbg!(edits);

//     let edits = dom.hard_diff(ScopeId(0));
//     dbg!(edits);

//     let edits = dom.hard_diff(ScopeId(0));
//     dbg!(edits);

//     let edits = dom.hard_diff(ScopeId(0));
//     dbg!(edits);
// }
