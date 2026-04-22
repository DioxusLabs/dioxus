use std::{fmt::Display, marker::PhantomData};

use dioxus::prelude::*;

// This test just checks that props compile with generics
// It will not actually run any code
#[test]
#[allow(unused)]
#[allow(non_snake_case)]
fn generic_props_compile() {
    fn app() -> Element {
        rsx! {
            TakesClone {
                value: "hello world"
            }
            TakesCloneManual {
                value: "hello world"
            }
            TakesCloneManualWhere {
                value: "hello world"
            }
            GenericFnWhereClause {
                value: "hello world"
            }
        }
    }

    #[component]
    fn TakesClone<T: Clone + PartialEq + 'static>(value: T) -> Element {
        rsx! {}
    }

    #[component]
    fn TakesCloneArc<T: PartialEq + 'static>(value: std::sync::Arc<T>) -> Element {
        rsx! {}
    }

    struct MyBox<T>(std::marker::PhantomData<T>);

    impl<T: Display> Clone for MyBox<T> {
        fn clone(&self) -> Self {
            MyBox(std::marker::PhantomData)
        }
    }

    impl<T: Display> PartialEq for MyBox<T> {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }

    #[component]
    #[allow(clippy::multiple_bound_locations)]
    fn TakesCloneMyBox<T: 'static>(value: MyBox<T>) -> Element
    where
        T: Display,
    {
        rsx! {}
    }

    #[derive(Props, Clone, PartialEq)]
    struct TakesCloneManualProps<T: Clone + PartialEq + 'static> {
        value: T,
    }

    fn TakesCloneManual<T: Clone + PartialEq>(props: TakesCloneManualProps<T>) -> Element {
        rsx! {}
    }

    #[derive(Props, Clone, PartialEq)]
    struct TakesCloneManualWhereProps<T>
    where
        T: Clone + PartialEq + 'static,
    {
        value: T,
    }

    fn TakesCloneManualWhere<T: Clone + PartialEq>(
        props: TakesCloneManualWhereProps<T>,
    ) -> Element {
        rsx! {}
    }

    #[derive(Props, Clone, PartialEq)]
    struct TakesCloneManualWhereWithOwnerProps<T>
    where
        T: Clone + PartialEq + 'static,
    {
        value: EventHandler<T>,
    }

    fn TakesCloneManualWhereWithOwner<T: Clone + PartialEq>(
        props: TakesCloneManualWhereWithOwnerProps<T>,
    ) -> Element {
        rsx! {}
    }

    #[component]
    fn GenericFnWhereClause<T>(value: T) -> Element
    where
        T: Clone + PartialEq + Display + 'static,
    {
        rsx! {
            p { "{value}" }
        }
    }

    #[derive(Props)]
    struct PropsWithOwnerOnlyRequiresStatic<T: 'static> {
        value: PhantomData<T>,
        trigger_owner: EventHandler<()>,
    }

    impl<T> PartialEq for PropsWithOwnerOnlyRequiresStatic<T> {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }

    impl<T> Clone for PropsWithOwnerOnlyRequiresStatic<T> {
        fn clone(&self) -> Self {
            PropsWithOwnerOnlyRequiresStatic {
                value: PhantomData,
                trigger_owner: self.trigger_owner,
            }
        }
    }

    fn ReadSignalNoCloneWithOwner<T>(props: PropsWithOwnerOnlyRequiresStatic<T>) -> Element {
        rsx! {}
    }

    #[component]
    fn ReadSignalNoClone<T>(value: ReadSignal<T>) -> Element
    where
        T: PartialEq + Display + 'static,
    {
        rsx! {
            p { "{value}" }
        }
    }
}
