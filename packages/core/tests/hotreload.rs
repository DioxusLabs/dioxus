use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_core::Mutation::{AppendChildren, LoadTemplate};

/// Swap out the template and get it back via the mutation
#[test]
fn hotreloads_template() {
    let old_rsx = rsx! { "A" };
    let name = old_rsx.as_ref().unwrap().template.get().name;

    let mut dom = VirtualDom::new_with_props(move |_| old_rsx.clone(), ());

    let new_template = Template {
        name,
        roots: &[TemplateNode::Text { text: "B" }],
        node_paths: &[],
        attr_paths: &[],
    };

    dom.replace_template(new_template);

    let muts = dom.rebuild_to_vec();

    // New template comes out
    assert_eq!(muts.templates.len(), 1);

    assert_eq!(
        muts.edits,
        [
            LoadTemplate {
                name: "packages/core/tests/hotreload.rs:8:19:0",
                index: 0,
                id: ElementId(1,),
            },
            AppendChildren { id: ElementId(0,), m: 1 },
        ]
    )
}
