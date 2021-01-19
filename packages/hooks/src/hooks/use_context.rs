// Naming this file use_context could be confusing. Not least to the IDE.
use crate::{get_current_scope, use_hook};
use std::any::TypeId;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::{iter, mem};
use yew::html;
use yew::html::{AnyScope, Scope};
use yew::{Children, Component, ComponentLink, Html, Properties};

type ConsumerCallback<T> = Box<dyn Fn(Rc<T>)>;
type UseContextOutput<T> = Option<Rc<T>>;

struct UseContext<T2: Clone + PartialEq + 'static> {
    provider_scope: Option<Scope<ContextProvider<T2>>>,
    current_context: Option<Rc<T2>>,
    callback: Option<Rc<ConsumerCallback<T2>>>,
}

pub fn use_context<T: Clone + PartialEq + 'static>() -> UseContextOutput<T> {
    let scope = get_current_scope()
        .expect("No current Scope. `use_context` can only be called inside function components");

    use_hook(
        // Initializer
        move || {
            let provider_scope = find_context_provider_scope::<T>(&scope);
            let current_context =
                with_provider_component(&provider_scope, |comp| Rc::clone(&comp.context));

            UseContext {
                provider_scope,
                current_context,
                callback: None,
            }
        },
        // Runner
        |hook, updater| {
            // setup a listener for the context provider to update us
            let listener = move |ctx: Rc<T>| {
                updater.callback(move |state: &mut UseContext<T>| {
                    state.current_context = Some(ctx);
                    true
                });
            };
            hook.callback = Some(Rc::new(Box::new(listener)));

            // Subscribe to the context provider with our callback
            let weak_cb = Rc::downgrade(hook.callback.as_ref().unwrap());
            with_provider_component(&hook.provider_scope, |comp| {
                comp.subscribe_consumer(weak_cb)
            });

            // Return the current state
            hook.current_context.clone()
        },
        // Cleanup
        |hook| {
            if let Some(cb) = hook.callback.take() {
                drop(cb);
            }
        },
    )
}

#[derive(Clone, PartialEq, Properties)]
pub struct ContextProviderProps<T: Clone + PartialEq> {
    pub context: T,
    pub children: Children,
}

pub struct ContextProvider<T: Clone + PartialEq + 'static> {
    context: Rc<T>,
    children: Children,
    consumers: RefCell<Vec<Weak<ConsumerCallback<T>>>>,
}

impl<T: Clone + PartialEq> ContextProvider<T> {
    /// Add the callback to the subscriber list to be called whenever the context changes.
    /// The consumer is unsubscribed as soon as the callback is dropped.
    fn subscribe_consumer(&self, mut callback: Weak<ConsumerCallback<T>>) {
        // consumers re-subscribe on every render. Try to keep the subscriber list small by reusing dead slots.
        let mut consumers = self.consumers.borrow_mut();
        for cb in consumers.iter_mut() {
            if cb.strong_count() == 0 {
                mem::swap(cb, &mut callback);
                return;
            }
        }

        // no slot to reuse, this is a new consumer
        consumers.push(callback);
    }

    /// Notify all subscribed consumers and remove dropped consumers from the list.
    fn notify_consumers(&mut self) {
        let context = &self.context;
        self.consumers.borrow_mut().retain(|cb| {
            if let Some(cb) = cb.upgrade() {
                cb(Rc::clone(context));
                true
            } else {
                false
            }
        });
    }
}

impl<T: Clone + PartialEq + 'static> Component for ContextProvider<T> {
    type Message = ();
    type Properties = ContextProviderProps<T>;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Self {
            children: props.children,
            context: Rc::new(props.context),
            consumers: RefCell::new(Vec::new()),
        }
    }

    fn update(&mut self, _msg: Self::Message) -> bool {
        true
    }

    fn change(&mut self, props: Self::Properties) -> bool {
        let should_render = if self.children == props.children {
            false
        } else {
            self.children = props.children;
            true
        };

        let new_context = Rc::new(props.context);
        if self.context != new_context {
            self.context = new_context;
            self.notify_consumers();
        }

        should_render
    }

    fn view(&self) -> Html {
        html! { <>{ self.children.clone() }</> }
    }
}

fn find_context_provider_scope<T: Clone + PartialEq + 'static>(
    scope: &AnyScope,
) -> Option<Scope<ContextProvider<T>>> {
    let expected_type_id = TypeId::of::<ContextProvider<T>>();
    iter::successors(Some(scope), |scope| scope.get_parent())
        .filter(|scope| scope.get_type_id() == &expected_type_id)
        .cloned()
        .map(AnyScope::downcast::<ContextProvider<T>>)
        .next()
}

fn with_provider_component<T, F, R>(
    provider_scope: &Option<Scope<ContextProvider<T>>>,
    f: F,
) -> Option<R>
where
    T: Clone + PartialEq,
    F: FnOnce(&ContextProvider<T>) -> R,
{
    provider_scope
        .as_ref()
        .and_then(|scope| scope.get_component().map(|comp| f(&*comp)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::{use_effect, use_ref, use_state};
    use crate::util::*;
    use crate::{FunctionComponent, FunctionProvider};
    use wasm_bindgen_test::*;
    use yew::prelude::*;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn use_context_scoping_works() {
        #[derive(Clone, Debug, PartialEq)]
        struct ExampleContext(String);
        struct UseContextFunctionOuter {}
        struct UseContextFunctionInner {}
        struct ExpectNoContextFunction {}
        type UseContextComponent = FunctionComponent<UseContextFunctionOuter>;
        type UseContextComponentInner = FunctionComponent<UseContextFunctionInner>;
        type ExpectNoContextComponent = FunctionComponent<ExpectNoContextFunction>;
        impl FunctionProvider for ExpectNoContextFunction {
            type TProps = ();

            fn run(_props: &Self::TProps) -> Html {
                if use_context::<ExampleContext>().is_some() {
                    yew::services::ConsoleService::log(&format!(
                        "Context should be None here, but was {:?}!",
                        use_context::<ExampleContext>().unwrap()
                    ));
                };
                return html! {
                    <div></div>
                };
            }
        }
        impl FunctionProvider for UseContextFunctionOuter {
            type TProps = ();

            fn run(_props: &Self::TProps) -> Html {
                type ExampleContextProvider = ContextProvider<ExampleContext>;
                return html! {
                    <div>
                        <ExampleContextProvider context=ExampleContext("wrong1".into())>
                            <div>{"ignored"}</div>
                        </ExampleContextProvider>
                        <ExampleContextProvider context=ExampleContext("wrong2".into())>
                            <ExampleContextProvider context=ExampleContext("correct".into())>
                                <ExampleContextProvider context=ExampleContext("wrong1".into())>
                                    <div>{"ignored"}</div>
                                </ExampleContextProvider>
                                <UseContextComponentInner />
                            </ExampleContextProvider>
                        </ExampleContextProvider>
                        <ExampleContextProvider context=ExampleContext("wrong3".into())>
                            <div>{"ignored"}</div>
                        </ExampleContextProvider>
                        <ExpectNoContextComponent />
                    </div>
                };
            }
        }
        impl FunctionProvider for UseContextFunctionInner {
            type TProps = ();

            fn run(_props: &Self::TProps) -> Html {
                let context = use_context::<ExampleContext>();
                return html! {
                    <div id="result">{ &context.unwrap().0 }</div>
                };
            }
        }

        let app: App<UseContextComponent> = yew::App::new();
        app.mount(yew::utils::document().get_element_by_id("output").unwrap());
        let result: String = obtain_result_by_id("result");
        assert_eq!("correct", result);
    }

    #[wasm_bindgen_test]
    fn use_context_works_with_multiple_types() {
        #[derive(Clone, Debug, PartialEq)]
        struct ContextA(u32);
        #[derive(Clone, Debug, PartialEq)]
        struct ContextB(u32);

        struct Test1Function;
        impl FunctionProvider for Test1Function {
            type TProps = ();

            fn run(_props: &Self::TProps) -> Html {
                assert_eq!(use_context::<ContextA>(), Some(Rc::new(ContextA(2))));
                assert_eq!(use_context::<ContextB>(), Some(Rc::new(ContextB(1))));

                return html! {};
            }
        }
        type Test1 = FunctionComponent<Test1Function>;

        struct Test2Function;
        impl FunctionProvider for Test2Function {
            type TProps = ();

            fn run(_props: &Self::TProps) -> Html {
                assert_eq!(use_context::<ContextA>(), Some(Rc::new(ContextA(0))));
                assert_eq!(use_context::<ContextB>(), Some(Rc::new(ContextB(1))));

                return html! {};
            }
        }
        type Test2 = FunctionComponent<Test2Function>;

        struct Test3Function;
        impl FunctionProvider for Test3Function {
            type TProps = ();

            fn run(_props: &Self::TProps) -> Html {
                assert_eq!(use_context::<ContextA>(), Some(Rc::new(ContextA(0))));
                assert_eq!(use_context::<ContextB>(), None);

                return html! {};
            }
        }
        type Test3 = FunctionComponent<Test3Function>;

        struct Test4Function;
        impl FunctionProvider for Test4Function {
            type TProps = ();

            fn run(_props: &Self::TProps) -> Html {
                assert_eq!(use_context::<ContextA>(), None);
                assert_eq!(use_context::<ContextB>(), None);

                return html! {};
            }
        }
        type Test4 = FunctionComponent<Test4Function>;

        struct TestFunction;
        impl FunctionProvider for TestFunction {
            type TProps = ();

            fn run(_props: &Self::TProps) -> Html {
                type ContextAProvider = ContextProvider<ContextA>;
                type ContextBProvider = ContextProvider<ContextB>;

                return html! {
                    <div>
                        <ContextAProvider context=ContextA(0)>
                            <ContextBProvider context=ContextB(1)>
                                <ContextAProvider context=ContextA(2)>
                                    <Test1/>
                                </ContextAProvider>
                                <Test2/>
                            </ContextBProvider>
                            <Test3/>
                        </ContextAProvider>
                        <Test4 />
                    </div>
                };
            }
        }
        type TestComponent = FunctionComponent<TestFunction>;

        let app: App<TestComponent> = yew::App::new();
        app.mount(yew::utils::document().get_element_by_id("output").unwrap());
    }

    #[wasm_bindgen_test]
    fn use_context_update_works() {
        #[derive(Clone, Debug, PartialEq)]
        struct MyContext(String);

        #[derive(Clone, Debug, PartialEq, Properties)]
        struct RenderCounterProps {
            id: String,
            children: Children,
        }

        struct RenderCounterFunction;
        impl FunctionProvider for RenderCounterFunction {
            type TProps = RenderCounterProps;

            fn run(props: &Self::TProps) -> Html {
                let counter = use_ref(|| 0);
                *counter.borrow_mut() += 1;
                log::info!("Render counter {:?}", counter);
                return html! {
                    <>
                        <div id=props.id.clone()>
                            { format!("total: {}", counter.borrow()) }
                        </div>
                        { props.children.clone() }
                    </>
                };
            }
        }
        type RenderCounter = FunctionComponent<RenderCounterFunction>;

        #[derive(Clone, Debug, PartialEq, Properties)]
        struct ContextOutletProps {
            id: String,
            #[prop_or_default]
            magic: usize,
        }
        struct ContextOutletFunction;
        impl FunctionProvider for ContextOutletFunction {
            type TProps = ContextOutletProps;

            fn run(props: &Self::TProps) -> Html {
                let counter = use_ref(|| 0);
                *counter.borrow_mut() += 1;

                let ctx = use_context::<Rc<MyContext>>().expect("context not passed down");
                log::info!("============");
                log::info!("ctx is {:#?}", ctx);
                log::info!("magic is {:#?}", props.magic);
                log::info!("outlet counter is {:#?}", ctx);
                log::info!("============");

                return html! {
                    <>
                        <div>{ format!("magic: {}\n", props.magic) }</div>
                        <div id=props.id.clone()>
                            { format!("current: {}, total: {}", ctx.0, counter.borrow()) }
                        </div>
                    </>
                };
            }
        }
        type ContextOutlet = FunctionComponent<ContextOutletFunction>;

        struct TestFunction;
        impl FunctionProvider for TestFunction {
            type TProps = ();

            fn run(_props: &Self::TProps) -> Html {
                type MyContextProvider = ContextProvider<Rc<MyContext>>;

                let (ctx, set_ctx) = use_state(|| MyContext("hello".into()));
                let rendered = use_ref(|| 0);

                // this is used to force an update specific to test-2
                let (magic_rc, set_magic) = use_state(|| 0);
                let magic: usize = *magic_rc;

                use_effect(move || {
                    let count = *rendered.borrow();
                    match count {
                        0 => {
                            set_ctx(MyContext("world".into()));
                            *rendered.borrow_mut() += 1;
                        }
                        1 => {
                            // force test-2 to re-render.
                            set_magic(1);
                            *rendered.borrow_mut() += 1;
                        }
                        2 => {
                            set_ctx(MyContext("hello world!".into()));
                            *rendered.borrow_mut() += 1;
                        }
                        _ => (),
                    };
                    || {}
                });

                return html! {
                    <MyContextProvider context=ctx>
                        <RenderCounter id="test-0">
                            <ContextOutlet id="test-1"/>
                            <ContextOutlet id="test-2" magic=magic/>
                        </RenderCounter>
                    </MyContextProvider>
                };
            }
        }
        type TestComponent = FunctionComponent<TestFunction>;

        wasm_logger::init(wasm_logger::Config::default());
        let app: App<TestComponent> = yew::App::new();
        app.mount(yew::utils::document().get_element_by_id("output").unwrap());

        // 1 initial render + 3 update steps
        assert_eq!(obtain_result_by_id("test-0"), "total: 4");

        // 1 initial + 2 context update
        assert_eq!(
            obtain_result_by_id("test-1"),
            "current: hello world!, total: 3"
        );

        // 1 initial + 1 context update + 1 magic update + 1 context update
        assert_eq!(
            obtain_result_by_id("test-2"),
            "current: hello world!, total: 4"
        );
    }
}
