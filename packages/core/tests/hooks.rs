use std::{cell::RefCell, rc::Rc};

use anyhow::{Context, Result};
use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;

type Shared<T> = Rc<RefCell<T>>;

#[test]
fn sample_refs() {

    // static App: FC<()> = |(cx, props)|{
    //     let div_ref = use_node_ref::<MyRef, _>(cx);

    //     cx.render(rsx! {
    //         div {
    //             style: { color: "red" },
    //             node_ref: div_ref,
    //             onmouseover: move |_| {
    //                 div_ref.borrow_mut().focus();
    //             },
    //         },
    //     })
    // };
}
