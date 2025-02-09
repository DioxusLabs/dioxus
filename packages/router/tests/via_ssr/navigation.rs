use dioxus::prelude::*;
use dioxus_core::NoOpMutations;
use std::sync::atomic::AtomicUsize;

// Regression test for <https://github.com/DioxusLabs/dioxus/issues/3235>
#[test]
fn layout_retains_state_after_navigation() {
    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();

    vdom.render_immediate(&mut NoOpMutations);
    let as_string = dioxus_ssr::render(&vdom);
    assert_eq!(as_string, "Other");
}

fn app() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

// Turn off rustfmt since we're doing layouts and routes in the same enum
#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    // Wrap Home in a Navbar Layout
    #[layout(NavBar)]
        // The default route is always "/" unless otherwise specified
        #[route("/")]
        Home {},
            
        #[route("/other")]
        Other {},
}

#[component]
fn NavBar() -> Element {
    static NAVBARS_CREATED: AtomicUsize = AtomicUsize::new(0);
    use_hook(|| {
        let navbars_created = NAVBARS_CREATED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        println!("creating navbar #{navbars_created}");
        if navbars_created > 0 {
            panic!("layouts should not be recreated when switching between two routes under the nav bar");
        }
    });

    // Queue an effect to navigate to the other route after rebuild_in_place
    use_effect(|| {
        router().push(Route::Other {});
    });

    rsx! {
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        "Home!"
    }
}

#[component]
fn Other() -> Element {
    rsx! {
        "Other"
    }
}
