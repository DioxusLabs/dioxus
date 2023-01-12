use dioxus::prelude::*;
use dioxus_native_core::{
    node_ref::{AttributeMask, NodeView},
    real_dom::RealDom,
    state::{ParentDepState, State},
    NodeMask, SendAnyMap,
};
use dioxus_native_core_macro::{sorted_str_slice, State};
use std::sync::{Arc, Mutex};
use tokio::time::sleep;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlablaState {}

/// Font style are inherited by default if not specified otherwise by some of the supported attributes.
impl ParentDepState for BlablaState {
    type Ctx = ();
    type DepState = (Self,);

    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(&sorted_str_slice!(["blabla",])));

    fn reduce<'a>(
        &mut self,
        _node: NodeView,
        _parent: Option<(&'a Self,)>,
        _ctx: &Self::Ctx,
    ) -> bool {
        false
    }
}

#[derive(Clone, State, Default, Debug)]
pub struct NodeState {
    #[parent_dep_state(blabla)]
    blabla: BlablaState,
}

mod dioxus_elements {
    macro_rules! builder_constructors {
        (
            $(
                $(#[$attr:meta])*
                $name:ident {
                    $(
                        $(#[$attr_method:meta])*
                        $fil:ident: $vil:ident,
                    )*
                };
            )*
        ) => {
            $(
                #[allow(non_camel_case_types)]
                $(#[$attr])*
                pub struct $name;

                impl $name {
                    pub const TAG_NAME: &'static str = stringify!($name);
                    pub const NAME_SPACE: Option<&'static str> = None;

                    $(
                        pub const $fil: (&'static str, Option<&'static str>, bool) = (stringify!($fil), None, false);
                    )*
                }

                impl GlobalAttributes for $name {}
            )*
        }
    }

    pub trait GlobalAttributes {}

    pub trait SvgAttributes {}

    builder_constructors! {
        blabla {

        };
    }
}

#[test]
fn native_core_is_okay() {
    use std::time::Duration;

    fn app(cx: Scope) -> Element {
        let colors = use_state(cx, || vec!["green", "blue", "red"]);
        let padding = use_state(cx, || 10);

        use_effect(cx, colors, |colors| async move {
            sleep(Duration::from_millis(1000)).await;
            colors.with_mut(|colors| colors.reverse());
        });

        use_effect(cx, padding, |padding| async move {
            sleep(Duration::from_millis(10)).await;
            padding.with_mut(|padding| {
                if *padding < 65 {
                    *padding += 1;
                } else {
                    *padding = 5;
                }
            });
        });

        let _big = colors[0];
        let _mid = colors[1];
        let _small = colors[2];

        cx.render(rsx! {
            blabla {}
        })
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        let rdom = Arc::new(Mutex::new(RealDom::<NodeState>::new()));
        let mut dom = VirtualDom::new(app);

        let muts = dom.rebuild();
        let (to_update, _diff) = rdom.lock().unwrap().apply_mutations(muts);

        let ctx = SendAnyMap::new();
        rdom.lock().unwrap().update_state(to_update, ctx);

        for _ in 0..10 {
            dom.wait_for_work().await;

            let mutations = dom.render_immediate();
            let (to_update, _diff) = rdom.lock().unwrap().apply_mutations(mutations);

            let ctx = SendAnyMap::new();
            rdom.lock().unwrap().update_state(to_update, ctx);
        }
    });
}
