use crate::use_hook;
use std::rc::Rc;

struct UseReducer<State> {
    current_state: Rc<State>,
}

/// This hook is an alternative to [`use_state`]. It is used to handle component's state and is used
/// when complex actions needs to be performed on said state.
///
/// For lazy initialization, consider using [`use_reducer_with_init`] instead.
///
/// # Example
/// ```rust
/// # use yew_functional::{function_component, use_reducer};
/// # use yew::prelude::*;
/// # use std::rc::Rc;
/// # use std::ops::DerefMut;
/// #
/// #[function_component(UseReducer)]
/// fn reducer() -> Html {
///     /// reducer's Action
///     enum Action {
///         Double,
///         Square,
///     }
///
///     /// reducer's State
///     struct CounterState {
///         counter: i32,
///     }
///
///     let (
///         counter, // the state
///         // function to update the state
///         // as the same suggests, it dispatches the values to the reducer function
///         dispatch
///     ) = use_reducer(
///         // the reducer function
///         |prev: Rc<CounterState>, action: Action| CounterState {
///             counter: match action {
///                 Action::Double => prev.counter * 2,
///                 Action::Square => prev.counter * prev.counter,
///             }
///         },
///         // initial state
///         CounterState { counter: 1 },
///     );
///
///    let double_onclick = {
///         let dispatch = Rc::clone(&dispatch);
///         Callback::from(move |_| dispatch(Action::Double))
///     };
///     let square_onclick = Callback::from(move |_| dispatch(Action::Square));
///
///     html! {
///         <>
///             <div id="result">{ counter.counter }</div>
///
///             <button onclick=double_onclick>{ "Double" }</button>
///             <button onclick=square_onclick>{ "Square" }</button>
///         </>
///     }
/// }
/// ```
pub fn use_reducer<Action: 'static, Reducer, State: 'static>(
    reducer: Reducer,
    initial_state: State,
) -> (Rc<State>, Rc<dyn Fn(Action)>)
where
    Reducer: Fn(Rc<State>, Action) -> State + 'static,
{
    use_reducer_with_init(reducer, initial_state, |a| a)
}

/// [`use_reducer`] but with init argument.
///
/// This is useful for lazy initialization where it is beneficial not to perform expensive
/// computation up-front
///
/// # Example
/// ```rust
/// # use yew_functional::{function_component, use_reducer_with_init};
/// # use yew::prelude::*;
/// # use std::rc::Rc;
/// #
/// #[function_component(UseReducerWithInit)]
/// fn reducer_with_init() -> Html {
///     struct CounterState {
///         counter: i32,
///     }
///     let (counter, dispatch) = use_reducer_with_init(
///         |prev: Rc<CounterState>, action: i32| CounterState {
///             counter: prev.counter + action,
///         },
///         0,
///         |initial: i32| CounterState {
///             counter: initial + 10,
///         },
///     );
///
///     html! {
///         <>
///             <div id="result">{counter.counter}</div>
///
///             <button onclick=Callback::from(move |_| dispatch(10))>{"Increment by 10"}</button>
///         </>
///     }
/// }
/// ```
pub fn use_reducer_with_init<
    Reducer,
    Action: 'static,
    State: 'static,
    InitialState: 'static,
    InitFn: 'static,
>(
    reducer: Reducer,
    initial_state: InitialState,
    init: InitFn,
) -> (Rc<State>, Rc<dyn Fn(Action)>)
where
    Reducer: Fn(Rc<State>, Action) -> State + 'static,
    InitFn: Fn(InitialState) -> State,
{
    let init = Box::new(init);
    let reducer = Rc::new(reducer);
    use_hook(
        move || UseReducer {
            current_state: Rc::new(init(initial_state)),
        },
        |s, updater| {
            let setter: Rc<dyn Fn(Action)> = Rc::new(move |action: Action| {
                let reducer = reducer.clone();
                // We call the callback, consumer the updater
                // Required to put the type annotations on Self so the method knows how to downcast
                updater.callback(move |state: &mut UseReducer<State>| {
                    let new_state = reducer(state.current_state.clone(), action);
                    state.current_state = Rc::new(new_state);
                    true
                });
            });

            let current = s.current_state.clone();
            (current, setter)
        },
        |_| {},
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::hooks::use_effect_with_deps;
    use crate::util::*;
    use crate::{FunctionComponent, FunctionProvider};
    use wasm_bindgen_test::*;
    use yew::prelude::*;
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn use_reducer_works() {
        struct UseReducerFunction {}
        impl FunctionProvider for UseReducerFunction {
            type TProps = ();
            fn run(_: &Self::TProps) -> Html {
                struct CounterState {
                    counter: i32,
                }
                let (counter, dispatch) = use_reducer_with_init(
                    |prev: std::rc::Rc<CounterState>, action: i32| CounterState {
                        counter: prev.counter + action,
                    },
                    0,
                    |initial: i32| CounterState {
                        counter: initial + 10,
                    },
                );

                use_effect_with_deps(
                    move |_| {
                        dispatch(1);
                        || {}
                    },
                    (),
                );
                return html! {
                    <div>
                        {"The test result is"}
                        <div id="result">{counter.counter}</div>
                        {"\n"}
                    </div>
                };
            }
        }
        type UseReducerComponent = FunctionComponent<UseReducerFunction>;
        let app: App<UseReducerComponent> = yew::App::new();
        app.mount(yew::utils::document().get_element_by_id("output").unwrap());
        let result = obtain_result();

        assert_eq!(result.as_str(), "11");
    }
}
