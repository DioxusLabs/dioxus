use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

/// Should push the text node onto the stack and modify it
#[test]
fn many_roots() {
    fn app() -> Element {
        rsx! {
            div {
                MyNav {}
                MyOutlet {}
            }
            MyFooter {}
        }
    }

    fn MyFooter() -> Element {
        rsx! {
            div { "footer" }
        }
    }

    fn MyNav() -> Element {
        rsx!(
            div { "trailing nav" }
            MySearch {}
        )
    }

    fn MySearch() -> Element {
        rsx!("search")
    }

    fn MyOutlet() -> Element {
        rsx!(
            if true {
                div {
                    "homepage"
                }
            }
        )
    }

    let mut dom = VirtualDom::new(app);
    let edits = dom.rebuild_to_vec();

    println!("{:#?}", edits.edits);

    let mut stack = vec![];

    for edit in edits.edits {
        println!("stack: {stack:?}\nedit\n{edit:?}\n ");
        match edit {
            LoadTemplate { template, index, id } => stack.push(id),
            ReplacePlaceholder { path, m } => stack.truncate(stack.len() - m),
            AppendChildren { id, m } => stack.truncate(stack.len() - m),

            AssignId { path, id } => todo!(),
            CreatePlaceholder { id } => todo!(),
            CreateTextNode { value, id } => todo!(),
            ReplaceWith { id, m } => todo!(),
            InsertAfter { id, m } => todo!(),
            InsertBefore { id, m } => todo!(),
            SetAttribute { name, ns, value, id } => todo!(),
            SetText { value, id } => todo!(),
            NewEventListener { name, id } => todo!(),
            RemoveEventListener { name, id } => todo!(),
            Remove { id } => todo!(),
            PushRoot { id } => todo!(),
        }
    }

    dbg!(stack);

    // assert_eq!(
    //     edits.edits,
    //     [
    //         LoadTemplate { index: 0, id: ElementId(1) },
    //         AppendChildren { m: 1, id: ElementId(0) },
    //     ]
    // )
}
