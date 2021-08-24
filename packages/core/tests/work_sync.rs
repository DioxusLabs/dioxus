//! Diffing is interruptible, but uses yield_now which is loop-pollable
//!
//! This means you can actually call it synchronously if you want.

use anyhow::{Context, Result};
use dioxus::{arena::SharedResources, diff::DiffMachine, prelude::*, scope::Scope};
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;
use futures_util::FutureExt;

#[test]
fn worksync() {
    static App: FC<()> = |cx| {
        cx.render(rsx! {
            div {"hello"}
        })
    };
    let mut dom = VirtualDom::new(App);

    let mut fut = dom.rebuild_async().boxed_local();

    let mutations = loop {
        let g = (&mut fut).now_or_never();
        if g.is_some() {
            break g.unwrap();
        }
    };

    dbg!(mutations);
}
