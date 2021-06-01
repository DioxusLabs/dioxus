use crate::use_hook;
use std::{cell::RefCell, rc::Rc};

/// This hook is used for obtaining a mutable reference to a stateful value.
/// Its state persists across renders.
///
/// It is important to note that you do not get notified of state changes.
/// If you need the component to be re-rendered on state change, consider using [`use_state`].
///
/// # Example
/// ```rust
/// # use yew_functional::{function_component, use_state, use_ref};
/// # use yew::prelude::*;
/// # use std::rc::Rc;
/// # use std::cell::RefCell;
/// # use std::ops::{Deref, DerefMut};
/// #
/// #[function_component(UseRef)]
/// fn ref_hook() -> Html {
///     let (message, set_message) = use_state(|| "".to_string());
///     let message_count = use_ref(|| 0);
///
///     let onclick = Callback::from(move |e| {
///         let window = yew::utils::window();
///
///         if *message_count.borrow_mut() > 3 {
///             window.alert_with_message("Message limit reached");
///         } else {
///             *message_count.borrow_mut() += 1;
///             window.alert_with_message("Message sent");
///         }
///     });
///
///     let onchange = Callback::from(move |e| {
///         if let ChangeData::Value(value) = e {
///             set_message(value)
///         }
///     });
///
///     html! {
///         <div>
///             <input onchange=onchange value=message />
///             <button onclick=onclick>{ "Send" }</button>
///         </div>
///     }
/// }
/// ```
pub fn use_ref<T: 'static>(initial_value: impl FnOnce() -> T + 'static) -> Rc<RefCell<T>> {
    use_hook(
        || Rc::new(RefCell::new(initial_value())),
        |state| state.clone(),
        |_| {},
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hooks::use_state,
        util::*,
        {FunctionComponent, FunctionProvider},
    };
    use std::ops::DerefMut;
    use wasm_bindgen_test::*;
    use yew::prelude::*;
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn use_ref_works() {
        struct UseRefFunction {}
        impl FunctionProvider for UseRefFunction {
            type TProps = ();

            fn run(_: &Self::TProps) -> Html {
                let ref_example = use_ref(|| 0);
                *ref_example.borrow_mut().deref_mut() += 1;
                let (counter, set_counter) = use_state(|| 0);
                if *counter < 5 {
                    set_counter(*counter + 1)
                }
                return html! {
                    <div>
                        {"The test output is: "}
                        <div id="result">{*ref_example.borrow_mut().deref_mut() > 4}</div>
                        {"\n"}
                    </div>
                };
            }
        }
        type UseRefComponent = FunctionComponent<UseRefFunction>;
        let app: App<UseRefComponent> = yew::App::new();
        app.mount(yew::utils::document().get_element_by_id("output").unwrap());

        let result = obtain_result();
        assert_eq!(result.as_str(), "true");
    }
}
