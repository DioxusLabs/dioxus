use dioxus::prelude::*;

#[test]
fn with_class() {
    #[derive(Routable, Clone, PartialEq, Debug)]
    enum ChildRoute {
        #[route("/")]
        ChildRoot {},
        #[route("/:not_static")]
        NotStatic { not_static: String },
    }

    #[derive(Routable, Clone, PartialEq, Debug)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
        #[child("/child")]
        Nested { child: ChildRoute },
    }

    #[component]
    fn Test() -> Element {
        unimplemented!()
    }

    #[component]
    fn Root() -> Element {
        unimplemented!()
    }

    #[component]
    fn ChildRoot() -> Element {
        unimplemented!()
    }

    #[component]
    fn NotStatic(not_static: String) -> Element {
        unimplemented!()
    }

    assert_eq!(
        Route::static_routes(),
        vec![
            Route::Root {},
            Route::Test {},
            Route::Nested {
                child: ChildRoute::ChildRoot {}
            },
        ],
    );
}
