#![allow(
    unused,
    dead_code,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case
)]

fn main() {}

use dioxus::prelude::*;
use std::rc::Rc;

static App: FC<()> = |cx| {
    let (selection, set_selection) = use_state(cx, || None as Option<usize>).classic();

    let body = match selection {
        Some(id) => rsx!(in cx, ReferenceItem { selected: *id }),
        None => rsx!(in cx, div { "Select an concept to explore" }),
    };

    cx.render(rsx! {
        div {
            ScrollSelector { onselect: move |id| set_selection(id)  }
            {body}
        }
    })
};

// this is its own component to stay memoized
#[derive(Props)]
struct ScrollSelectorProps<'a> {
    onselect: &'a dyn Fn(Option<usize>),
}

fn ScrollSelector<'a>(cx: Context<'a, ScrollSelectorProps>) -> VNode<'a> {
    let selection_list = (&REFERENCES).iter().enumerate().map(|(id, _)| {
        rsx! {
            li {
                h3 {}
            }
        }
    });
    cx.render(rsx! {
        div {
            h1 {""}
            ul {
                {selection_list}
                button {
                    onclick: move |_| (cx.onselect)(Some(10))
                }
            }
        }
    })
}

#[derive(PartialEq, Props)]
struct ReferenceItemProps {
    selected: usize,
}

static ReferenceItem: FC<ReferenceItemProps> = |cx| {
    let (caller, name, code) = REFERENCES[cx.selected];

    // Create the component using the factory API directly
    let caller_node = LazyNodes::new(move |f| f.component(caller, (), None, &[]));

    cx.render(rsx! {
        div {
            // Source of the left, rendered on the right
            div {
                code { "{code}" }
            }
            div {
                {caller_node}
            }
        }
    })
};

use reference::REFERENCES;
mod reference {
    mod antipatterns;
    mod basics;
    mod children;
    mod conditional_rendering;
    mod controlled_inputs;
    mod custom_elements;
    mod empty;
    mod fragments;
    mod global_css;
    mod inline_styles;
    mod iterators;
    mod listener;
    mod memo;
    mod noderefs;
    mod signals;
    mod spreadpattern;
    mod statemanagement;
    mod suspense;
    mod task;
    mod testing;
    mod tostring;
    use super::*;
    pub static REFERENCES: &[(FC<()>, &str, &str)] = &[
        (
            basics::Example,
            "Basics",
            include_str!("./reference/basics.rs"),
        ),
        (
            children::Example,
            "Children",
            include_str!("./reference/children.rs"),
        ),
        (
            conditional_rendering::Example,
            "Conditional Rendering",
            include_str!("./reference/conditional_rendering.rs"),
        ),
        // TODO
        (
            controlled_inputs::Example,
            "Controlled Inputs",
            include_str!("./reference/controlled_inputs.rs"),
        ),
        (
            empty::Example,
            "empty",
            include_str!("./reference/empty.rs"),
        ),
        (
            custom_elements::Example,
            "Custom Elements & Web Components",
            include_str!("./reference/custom_elements.rs"),
        ),
        (
            fragments::Example,
            "Fragments",
            include_str!("./reference/fragments.rs"),
        ),
        (
            iterators::Example,
            "Iterators",
            include_str!("./reference/iterators.rs"),
        ),
        (
            global_css::Example,
            "Global CSS",
            include_str!("./reference/global_css.rs"),
        ),
        (
            inline_styles::Example,
            "Inline Styles",
            include_str!("./reference/inline_styles.rs"),
        ),
        (
            listener::Example,
            "Listener",
            include_str!("./reference/listener.rs"),
        ),
        (memo::Example, "Memo", include_str!("./reference/memo.rs")),
        (
            spreadpattern::Example,
            "Spread Pattern",
            include_str!("./reference/spreadpattern.rs"),
        ),
        (
            suspense::Example,
            "Suspense",
            include_str!("./reference/suspense.rs"),
        ),
        (task::Example, "Task", include_str!("./reference/task.rs")),
        (
            tostring::Example,
            "Tostring",
            include_str!("./reference/tostring.rs"),
        ),
        (
            antipatterns::Example,
            "Anti-patterns",
            include_str!("./reference/antipatterns.rs"),
        ),
        /*
            TODO! these showcase features that aren't quite ready
        */
        (
            signals::Example,
            "Signals",
            include_str!("./reference/signals.rs"),
        ),
        (
            noderefs::Example,
            "NodeRefs",
            include_str!("./reference/noderefs.rs"),
        ),
        (
            statemanagement::Example,
            "State Management",
            include_str!("./reference/statemanagement.rs"),
        ),
        (
            testing::Example,
            "Testing",
            include_str!("./reference/testing.rs"),
        ),
    ];
}
