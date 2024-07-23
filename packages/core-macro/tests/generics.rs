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
        }
    }

    #[component]
    fn TakesClone<T: Clone + PartialEq + 'static>(value: T) -> Element {
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
}
