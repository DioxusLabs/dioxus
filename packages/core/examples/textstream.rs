use dioxus::virtual_dom::VirtualDom;
use dioxus_core::prelude::*;
fn main() {
    let mut dom = VirtualDom::new(App);
    let edits = dom.rebuild().unwrap();
    dbg!(edits);
}

static App: FC<()> = |ctx| {
    //
    ctx.render(rsx! {
        div {
            "abc"
            "123"
        }
    })
};

static Fragment: FC<()> = |ctx| {
    //

    let children = ctx.children();
    ctx.render(LazyNodes::new(move |c: &NodeCtx| {
        //
        let frag = c.bump().alloc(VFragment::new(None, children));
        VNode::Fragment(frag)
    }))
};
