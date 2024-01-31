use dioxus::prelude::*;
use dioxus_native_core::prelude::*;
use dioxus_native_core_macro::partial_derive_state;
use shipyard::Component;
use tokio::time::sleep;

#[derive(Debug, Clone, PartialEq, Eq, Default, Component)]
pub struct BlablaState {
    count: usize,
}

#[partial_derive_state]
impl State for BlablaState {
    type ParentDependencies = (Self,);
    type ChildDependencies = ();
    type NodeDependencies = ();

    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new()
        .with_attrs(AttributeMaskBuilder::Some(&["blabla"]))
        .with_element();

    fn update<'a>(
        &mut self,
        _: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
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
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> Self {
        let mut myself = Self::default();
        myself.update(node_view, node, parent, children, context);
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
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    fn app() -> Element {
        let colors = use_signal(|| vec!["green", "blue", "red"]);
        let padding = use_signal(|| 10);

        use_effect(colors, |colors| async move {
            sleep(Duration::from_millis(1000)).await;
            colors.with_mut(|colors| colors.reverse());
        });

        use_effect(padding, |padding| async move {
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

        rsx! {
            blabla {}
        }
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        let rdom = Arc::new(Mutex::new(RealDom::new([BlablaState::to_type_erased()])));
        let mut dioxus_state = DioxusState::create(&mut rdom.lock().unwrap());
        let mut dom = VirtualDom::new(app);

        let mutations = dom.rebuild();
        dioxus_state.apply_mutations(&mut rdom.lock().unwrap(), mutations);

        let ctx = SendAnyMap::new();
        rdom.lock().unwrap().update_state(ctx);

        for _ in 0..10 {
            dom.wait_for_work().await;

            let mutations = dom.render_immediate();
            dioxus_state.apply_mutations(&mut rdom.lock().unwrap(), mutations);

            let ctx = SendAnyMap::new();
            rdom.lock().unwrap().update_state(ctx);
        }
    });
}
