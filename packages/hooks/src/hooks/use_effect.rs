use crate::use_hook;
use std::{borrow::Borrow, rc::Rc};

struct UseEffect<Destructor> {
    destructor: Option<Box<Destructor>>,
}

/// This hook is used for hooking into the component's lifecycle.
///
/// # Example
/// ```rust
/// # use yew_functional::{function_component, use_effect, use_state};
/// # use yew::prelude::*;
/// # use std::rc::Rc;
/// #
/// #[function_component(UseEffect)]
/// fn effect() -> Html {
///     let (counter, set_counter) = use_state(|| 0);
///
///     let counter_one = counter.clone();
///     use_effect(move || {
///         // Make a call to DOM API after component is rendered
///         yew::utils::document().set_title(&format!("You clicked {} times", counter_one));
///
///         // Perform the cleanup
///         || yew::utils::document().set_title(&format!("You clicked 0 times"))
///     });
///
///     let onclick = {
///         let counter = Rc::clone(&counter);
///         Callback::from(move |_| set_counter(*counter + 1))
///     };
///
///     html! {
///         <button onclick=onclick>{ format!("Increment to {}", counter) }</button>
///     }
/// }
/// ```
pub fn use_effect<Destructor>(callback: impl FnOnce() -> Destructor + 'static)
where
    Destructor: FnOnce() + 'static,
{
    let callback = Box::new(callback);
    use_hook(
        move || {
            let effect: UseEffect<Destructor> = UseEffect { destructor: None };
            effect
        },
        |_, updater| {
            // Run on every render
            updater.post_render(move |state: &mut UseEffect<Destructor>| {
                if let Some(de) = state.destructor.take() {
                    de();
                }
                let new_destructor = callback();
                state.destructor.replace(Box::new(new_destructor));
                false
            });
        },
        |hook| {
            if let Some(destructor) = hook.destructor.take() {
                destructor()
            }
        },
    )
}

struct UseEffectDeps<Destructor, Dependents> {
    destructor: Option<Box<Destructor>>,
    deps: Rc<Dependents>,
}

/// This hook is similar to [`use_effect`] but it accepts dependencies.
///
/// Whenever the dependencies are changed, the effect callback is called again.
/// To detect changes, dependencies must implement `PartialEq`.
/// Note that the destructor also runs when dependencies change.
pub fn use_effect_with_deps<Callback, Destructor, Dependents>(callback: Callback, deps: Dependents)
where
    Callback: FnOnce(&Dependents) -> Destructor + 'static,
    Destructor: FnOnce() + 'static,
    Dependents: PartialEq + 'static,
{
    let deps = Rc::new(deps);
    let deps_c = deps.clone();

    use_hook(
        move || {
            let destructor: Option<Box<Destructor>> = None;
            UseEffectDeps {
                destructor,
                deps: deps_c,
            }
        },
        move |_, updater| {
            updater.post_render(move |state: &mut UseEffectDeps<Destructor, Dependents>| {
                if state.deps != deps {
                    if let Some(de) = state.destructor.take() {
                        de();
                    }
                    let new_destructor = callback(deps.borrow());
                    state.deps = deps;
                    state.destructor.replace(Box::new(new_destructor));
                } else if state.destructor.is_none() {
                    state
                        .destructor
                        .replace(Box::new(callback(state.deps.borrow())));
                }
                false
            });
        },
        |hook| {
            if let Some(destructor) = hook.destructor.take() {
                destructor()
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use crate::hooks::{use_effect_with_deps, use_ref, use_state};
    use crate::util::*;
    use crate::{FunctionComponent, FunctionProvider};
    use std::ops::Deref;
    use std::ops::DerefMut;
    use std::rc::Rc;
    use wasm_bindgen_test::*;
    use yew::prelude::*;
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn use_effect_works_many_times() {
        struct UseEffectFunction {}
        impl FunctionProvider for UseEffectFunction {
            type TProps = ();

            fn run(_: &Self::TProps) -> Html {
                let (counter, set_counter) = use_state(|| 0);
                let counter_clone = counter.clone();

                use_effect_with_deps(
                    move |_| {
                        if *counter_clone < 4 {
                            set_counter(*counter_clone + 1);
                        }
                        || {}
                    },
                    *counter,
                );

                return html! {
                    <div>
                        {"The test result is"}
                        <div id="result">{counter}</div>
                        {"\n"}
                    </div>
                };
            }
        }

        type UseEffectComponent = FunctionComponent<UseEffectFunction>;
        let app: App<UseEffectComponent> = yew::App::new();
        app.mount(yew::utils::document().get_element_by_id("output").unwrap());
        let result = obtain_result();
        assert_eq!(result.as_str(), "4");
    }

    #[wasm_bindgen_test]
    fn use_effect_works_once() {
        struct UseEffectFunction {}
        impl FunctionProvider for UseEffectFunction {
            type TProps = ();

            fn run(_: &Self::TProps) -> Html {
                let (counter, set_counter) = use_state(|| 0);
                let counter_clone = counter.clone();

                use_effect_with_deps(
                    move |_| {
                        set_counter(*counter_clone + 1);
                        || panic!("Destructor should not have been called")
                    },
                    (),
                );

                return html! {
                    <div>
                        {"The test result is"}
                        <div id="result">{counter}</div>
                        {"\n"}
                    </div>
                };
            }
        }
        type UseEffectComponent = FunctionComponent<UseEffectFunction>;
        let app: App<UseEffectComponent> = yew::App::new();
        app.mount(yew::utils::document().get_element_by_id("output").unwrap());
        let result = obtain_result();
        assert_eq!(result.as_str(), "1");
    }

    #[wasm_bindgen_test]
    fn use_effect_refires_on_dependency_change() {
        struct UseEffectFunction {}
        impl FunctionProvider for UseEffectFunction {
            type TProps = ();

            fn run(_: &Self::TProps) -> Html {
                let number_ref = use_ref(|| 0);
                let number_ref_c = number_ref.clone();
                let number_ref2 = use_ref(|| 0);
                let number_ref2_c = number_ref2.clone();
                let arg = *number_ref.borrow_mut().deref_mut();
                let (_, set_counter) = use_state(|| 0);
                use_effect_with_deps(
                    move |dep| {
                        let mut ref_mut = number_ref_c.borrow_mut();
                        let inner_ref_mut = ref_mut.deref_mut();
                        if *inner_ref_mut < 1 {
                            *inner_ref_mut += 1;
                            assert_eq!(dep, &0);
                        } else {
                            assert_eq!(dep, &1);
                        }
                        set_counter(10); // we just need to make sure it does not panic
                        move || {
                            set_counter(11);
                            *number_ref2_c.borrow_mut().deref_mut() += 1;
                        }
                    },
                    arg,
                );
                return html! {
                    <div>
                        {"The test result is"}
                        <div id="result">{*number_ref.borrow_mut().deref_mut()}{*number_ref2.borrow_mut().deref_mut()}</div>
                        {"\n"}
                    </div>
                };
            }
        }
        type UseEffectComponent = FunctionComponent<UseEffectFunction>;
        let app: App<UseEffectComponent> = yew::App::new();
        app.mount(yew::utils::document().get_element_by_id("output").unwrap());
        let result: String = obtain_result();

        assert_eq!(result.as_str(), "11");
    }

    #[wasm_bindgen_test]
    fn use_effect_destroys_on_component_drop() {
        struct UseEffectFunction {}
        struct UseEffectWrapper {}
        #[derive(Properties, Clone)]
        struct WrapperProps {
            destroy_called: Rc<dyn Fn()>,
        }
        impl PartialEq for WrapperProps {
            fn eq(&self, _other: &Self) -> bool {
                false
            }
        }
        #[derive(Properties, Clone)]
        struct FunctionProps {
            effect_called: Rc<dyn Fn()>,
            destroy_called: Rc<dyn Fn()>,
        }
        impl PartialEq for FunctionProps {
            fn eq(&self, _other: &Self) -> bool {
                false
            }
        }
        type UseEffectComponent = FunctionComponent<UseEffectFunction>;
        type UseEffectWrapperComponent = FunctionComponent<UseEffectWrapper>;
        impl FunctionProvider for UseEffectFunction {
            type TProps = FunctionProps;

            fn run(props: &Self::TProps) -> Html {
                let effect_called = props.effect_called.clone();
                let destroy_called = props.destroy_called.clone();
                use_effect_with_deps(
                    move |_| {
                        effect_called();
                        move || destroy_called()
                    },
                    (),
                );
                return html! {};
            }
        }
        impl FunctionProvider for UseEffectWrapper {
            type TProps = WrapperProps;

            fn run(props: &Self::TProps) -> Html {
                let (show, set_show) = use_state(|| true);
                if *show {
                    let effect_called: Rc<dyn Fn()> = Rc::new(move || set_show(false));
                    return html! {
                        <UseEffectComponent destroy_called=props.destroy_called.clone() effect_called=effect_called />
                    };
                } else {
                    return html! {
                        <div>{"EMPTY"}</div>
                    };
                }
            }
        }
        let app: App<UseEffectWrapperComponent> = yew::App::new();
        let destroy_counter = Rc::new(std::cell::RefCell::new(0));
        let destroy_counter_c = destroy_counter.clone();
        app.mount_with_props(
            yew::utils::document().get_element_by_id("output").unwrap(),
            WrapperProps {
                destroy_called: Rc::new(move || *destroy_counter_c.borrow_mut().deref_mut() += 1),
            },
        );
        assert_eq!(1, *destroy_counter.borrow().deref());
    }
}
