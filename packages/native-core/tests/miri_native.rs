use dioxus::prelude::*;
use dioxus_core::*;
use dioxus_native_core::{
    node_ref::{AttributeMask, AttributeMaskBuilder, NodeMaskBuilder, NodeView},
    real_dom::RealDom,
    Dependancy, NodeMask, Pass, SendAnyMap,
};
use std::cell::Cell;
use std::sync::{Arc, Mutex};
use tokio::time::sleep;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlablaState {
    count: usize,
}

impl Pass for BlablaState {
    type ParentDependencies = (Self,);
    type ChildDependencies = ();
    type NodeDependencies = ();

    const NODE_MASK: NodeMaskBuilder = NodeMaskBuilder::new()
        .with_attrs(AttributeMaskBuilder::Some(&["blabla"]))
        .with_element();

    fn pass<'a>(
        &mut self,
        node_view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
        _: &SendAnyMap,
    ) -> bool {
        if let Some((parent,)) = parent {
            if parent.count != 0 {
                self.count += 1;
            }
        }
        true
    }

    fn create<'a>(
        node_view: NodeView<()>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
        context: &SendAnyMap,
    ) -> Self {
        let mut myself = Self::default();
        myself.pass(node_view, node, parent, children, context);
        myself
    }
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
        let rdom = Arc::new(Mutex::new(RealDom::new(Box::new([
            BlablaState::to_type_erased(),
        ]))));
        let mut dom = VirtualDom::new(app);

        let muts = dom.rebuild();
        rdom.lock().unwrap().apply_mutations(muts);

        let ctx = SendAnyMap::new();
        rdom.lock().unwrap().update_state(ctx, false);

        for _ in 0..10 {
            dom.wait_for_work().await;

            let mutations = dom.render_immediate();
            rdom.lock().unwrap().apply_mutations(mutations);

            let ctx = SendAnyMap::new();
            rdom.lock().unwrap().update_state(ctx, false);
        }
    });
}
