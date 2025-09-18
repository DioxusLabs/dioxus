/*
parse out URL params
rest need to implement axum's FromRequest / extract
body: String
body: Bytes
payload: T where T: Deserialize (auto to Json, can wrap in other codecs)
extra items get merged as body, unless theyre also extractors?
hoist up FromRequest objects if they're just bounds
no State<T> extractors, use ServerState instead?

if there's a single trailing item, it's used as the body?

or, an entirely custom system, maybe based on names?
or, hoist up FromRequest objects into the signature?
*/

/*

an fn that returns an IntoFuture / async fn
- is clearer that it's an async fn....
- still shows up as a function
- can guard against being called on the client with IntoFuture?
- can be used as a handler directly
- requires a trait to be able to mess with it
- codegen for handling inputs seems more straightforward?

a static that implements Deref to a function pointer
- can guard against being called on the client
- can be used as a handler directly
- has methods on the static itself (like .path(), .method()) as well as the result
- does not show up as a proper function in docs
- callable types are a weird thing to do. deref is always weird to overload
- can have a builder API!

qs:
- should we even make it so you can access its props directly?
*/

fn main() {}

mod test_real_into_future {
    use std::prelude::rust_2024::Future;

    use anyhow::Result;
    use dioxus::prelude::dioxus_server;
    use dioxus_fullstack::{post, ServerFnEncoder, ServerFnError};

    #[derive(serde::Serialize, serde::Deserialize)]
    struct MyStuff {
        alpha: String,
        beta: String,
    }

    #[post("/my_path")]
    async fn do_thing_simple(data: MyStuff) -> Result<String> {
        todo!()
    }

    #[post("/my_path")]
    async fn do_thing_not_client(data: MyStuff) -> String {
        todo!()
    }

    async fn it_works() {
        let res: ServerFnEncoder<Result<String>> = do_thing_simple(MyStuff {
            alpha: "hello".into(),
            beta: "world".into(),
        });

        do_thing_not_client(MyStuff {
            alpha: "hello".into(),
            beta: "world".into(),
        })
        .await;
    }
}

/// This verifies the approach of our impl system.
mod real_impl {
    use std::prelude::rust_2024::Future;

    use anyhow::Result;
    use dioxus::prelude::dioxus_server;
    use dioxus_fullstack::{post, ServerFnError};

    #[derive(serde::Serialize, serde::Deserialize)]
    struct MyStuff {
        alpha: String,
        beta: String,
    }

    #[post("/my_path")]
    async fn do_thing_simple(data: MyStuff) -> Result<String> {
        todo!()
    }

    #[post("/my_path")]
    async fn do_thing_simple_with_real_err(data: MyStuff) -> Result<String, ServerFnError> {
        todo!()
    }

    #[post("/my_path/{id}/{r}/?a&b")]
    async fn do_thing(
        id: i32,
        r: i32,
        a: i32,
        b: String,
        alpha: String,
        beta: String,
    ) -> Result<String> {
        todo!()
    }

    async fn do_thing2(id: i32, b: String) -> Result<String> {
        todo!()
    }

    async fn it_works() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct MyParams {
            id: i32,
            b: String,
        }

        let res = reqwest::Client::new()
            .post("http://localhost:8080/my_path/5")
            .query(&MyParams {
                id: 5,
                b: "hello".into(),
            })
            .send()
            .await;

        match res {
            Ok(_) => todo!(),
            Err(err) => todo!(),
        }

        // do_thing2(id, b)
        // do_thing.path();
        // do_thing(id, b)
    }
}

mod modified_async_fn_sugar {
    use dioxus_fullstack::ServerFnError;

    async fn it_works() {
        // let mut res = demo1().await;
        let res = demo1();

        let res = demo2().await;
    }

    #[must_use = "futures do nothing unless you `.await` or poll them"]
    struct SrvFuture<Out> {
        _phant: std::marker::PhantomData<Out>,
    }

    impl<O, E> std::future::Future for SrvFuture<Result<O, E>>
    where
        E: Into<ServerFnError>,
    {
        type Output = i32;
        fn poll(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            todo!()
        }
    }

    trait Blah {
        type Out;
    }
    impl Blah for i32 {
        type Out = String;
    }

    fn demo1() -> SrvFuture<i32> {
        todo!()
    }

    fn demo2() -> SrvFuture<Result<i32, ServerFnError>> {
        todo!()
    }
}

/// this approach generates an fn() that returns an impl Future (basically a suped-up async fn gen)
/// - can be used as an axum handler directly? need to implement the trait I think?
/// - can have a builder API for extra options when creating the server query (.await fires off the call)
/// - calling invalid server_fns from the client is a runtime error, not a compile error -> maybe we could make bounds apply with cfg flag?
///     - conditional bounds are less useful if "server" shows up in your default features, won't see the bounds when writing code
/// - can call the handlers from the server
/// - no way AFAIK to get the path/method/etc from the function itself
/// - shows up in docs as a normal function, which is nice
/// - can be generic, which is good for composition.
///
/// potential improvements:
/// - a custom generic on ServerResponse can encode extra info about the server fn, like its path, method, etc
/// - or a custom ServerResponse object that implements Future can also work, even as an axum handler and a generic.
/// - just makes it a bit hard to put a bunch of them into a single collection, but might not be a big deal?
mod approach_with_fn {
    use anyhow::Result;
    use axum::{extract::State, Router};
    use dioxus_fullstack::DioxusServerState;
    use http::Method;

    trait Provider {}
    impl Provider for DioxusServerState {}
    fn it_works() {
        let router: Router<DioxusServerState> = axum::Router::new()
            .route(
                "/my_path/:id",
                axum::routing::post(my_server_fn::<DioxusServerState>),
            )
            .route(
                "/my_path2/:id",
                axum::routing::post(my_server_fn2::<DioxusServerState>),
            )
            .with_state(DioxusServerState::default());

        let res1 = my_server_fn2::<DioxusServerState>.path();
        let res2 = demo1.path();
        // let res = ServerFnInfo::path(&my_server_fn2::<DioxusServerState>);
    }

    async fn my_server_fn<T: Provider>(state: State<T>, b: String) -> String {
        todo!()
    }

    fn my_server_fn2<T: Provider>(state: State<T>, b: String) -> MyServerFn2Action<String> {
        MyServerFn2Action(std::marker::PhantomData::<String>)
    }

    trait ServerFnInfo<O, M> {
        const PATH: &'static str;
        const METHOD: Method;
        fn path(&self) -> &'static str {
            Self::PATH
        }
        fn method(&self) -> Method {
            Self::METHOD
        }
    }
    impl<F, O, G: ServerFnAction<O>> ServerFnInfo<O, (G,)> for F
    where
        F: Fn() -> G,
    {
        const PATH: &'static str = G::PATH;
        const METHOD: Method = G::METHOD;
        fn path(&self) -> &'static str {
            Self::PATH
        }
        fn method(&self) -> Method {
            Self::METHOD
        }
    }
    impl<A, B, F, O, G: ServerFnAction<O>> ServerFnInfo<O, (A, B, G)> for F
    where
        F: Fn(A, B) -> G,
    {
        const PATH: &'static str = G::PATH;
        const METHOD: Method = G::METHOD;
        fn path(&self) -> &'static str {
            Self::PATH
        }
        fn method(&self) -> Method {
            Self::METHOD
        }
    }

    /// Represents the output object of any annotated server function.
    /// Gives us metadata about the server function itself, as well as extensions for modifying
    /// the request before awaiting it (such as adding headers).
    ///
    /// Can deref directly to interior future to await it.
    /// Is cool because the output type can impl IntoResponse for axum handlers.
    ///
    /// Can be generic!
    trait ServerFnAction<Out> {
        const PATH: &'static str;
        const METHOD: Method;
    }

    struct MyServerFn2Action<T>(std::marker::PhantomData<T>);

    impl<T> ServerFnAction<String> for MyServerFn2Action<T> {
        const PATH: &'static str = "/my_path2/:id";
        const METHOD: Method = Method::POST;
    }
    impl<T> std::future::Future for MyServerFn2Action<T> {
        type Output = T;

        fn poll(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            todo!()
        }
    }

    fn demo1() -> MyServerFn2Action<Result<String>> {
        todo!()
    }

    fn demo2() -> ServerResponse<ServerFn2Endpoint<Result<String>>> {
        todo!()
    }

    struct ServerFn2Endpoint<R> {
        _phant: std::marker::PhantomData<R>,
    }
    struct ServerResponse<R> {
        _phant: std::marker::PhantomData<R>,
    }
    trait Endpoint {
        type Output;
        const PATH: &'static str;
        const METHOD: Method;
    }
    impl<T> std::future::Future for ServerResponse<T>
    where
        T: Endpoint,
    {
        type Output = T::Output;

        fn poll(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            todo!()
        }
    }
    impl<T> Endpoint for ServerFn2Endpoint<T> {
        type Output = T;
        const PATH: &'static str = "/my_path2/:id";
        const METHOD: Method = Method::POST;
    }

    async fn it_works_3() {
        let res = demo2().await;
    }
}

/// this approach uses a generated static to prevent you from calling axum-only handlers on the client
/// - generates a static with the path, method, and axum handler
/// - conditionally implements Deref to a function pointer for client calls
/// - has methods on the static for path, method, etc (nice!)
/// - can maybe be used as an axum handler directly? need to implement the trait I think?
/// - can have a builder API for extra options when creating the server query (.await fires off the call)
/// - the error from trying to call it on the client is a bit weird (says the static needs to be a function)
/// - how does calling from the server work? can they call each other? would you normally even do that?
/// - I don't think it can be generic?
/// - how do we accept user state? a State<T> extractor?
/// - we can choose to not generate the inventory submit if a generic is provided?
mod approach_with_static {

    use std::{ops::Deref, pin::Pin, prelude::rust_2024::Future, process::Output};

    use axum::{
        extract::{Request, State},
        response::Response,
        Router,
    };
    use dioxus_fullstack::{DioxusServerState, EncodeRequest, FetchRequest, ServerFnEncoder};
    use http::Method;
    use serde::de::DeserializeOwned;
    use tokio::task::JoinHandle;

    #[allow(non_upper_case_globals)]
    async fn it_works() {
        fn axum_router_with_server_fn() {
            let router: Router<DioxusServerState> = axum::Router::new()
                .route_service(do_thing_on_server.path, do_thing_on_server.clone())
                .with_state(DioxusServerState::default());
        }

        impl<T: FnMarker> tower::Service<Request> for ServerFnStatic<T> {
            type Response = axum::response::Response;
            type Error = std::convert::Infallible;
            type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

            fn poll_ready(
                &mut self,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                todo!()
            }

            fn call(&mut self, req: Request) -> Self::Future {
                todo!()
            }
        }

        /// A server function that can be called from the client or the server.
        ///
        /// This is a demo of what the generated code might look like.
        static do_thing_on_server: ServerFnStatic<((i32, String), anyhow::Result<String>)> = {
            /// A server function that can be called from the client or the server.
            ///
            /// This is a demo of what the generated code might look like.
            ///
            /// signature tokens are forwarded to this for better autocomplete
            async fn do_thing_on_server(a: i32, b: String) -> anyhow::Result<String> {
                todo!()
            }

            #[cfg(feature = "server")]
            inventory::submit! {
                ServerFunctionData {
                    path: "/some/path",
                    method: Method::POST,
                    on_server: |state, req| {
                        // state.spawn(async move {
                        tokio::spawn(async move {

                            let (parts, body) = req.into_parts();

                            todo!()
                        })
                        // })
                    },
                }
            };

            ServerFnStatic {
                path: "/some/path",
                method: Method::POST,
                on_client: |a, b| {
                    ServerResponse::new(async move {
                        // let res = (&&&&&&&&&&&&&&Client::<(i32, String)>::new())
                        //     .fetch::<String>(EncodeState::default(), (a, b))
                        //     .await;

                        todo!()
                    })
                },
            }
        };

        async fn do_thing_on_server2(id: i32, b: String) -> anyhow::Result<String> {
            todo!()
        }

        // We can get the path and method from the static
        let path = do_thing_on_server.path();

        let res = do_thing_on_server(5, "hello".to_string());
        let res = do_thing_on_server(5, "hello".to_string()).await;
    }

    inventory::collect!(ServerFnStatic<()>);
    inventory::collect!(ServerFunctionData);

    struct ServerFunctionData {
        path: &'static str,
        method: Method,
        on_server: fn(State<DioxusServerState>, Request) -> JoinHandle<Response>,
    }

    struct ServerFnStatic<Mark: FnMarker = ()> {
        path: &'static str,
        method: Method,
        on_client: Mark::UserCallable,
        // on_server: fn(State<DioxusServerState>, Request) -> JoinHandle<Response>,
    }
    impl<T: FnMarker> Clone for ServerFnStatic<T> {
        fn clone(&self) -> Self {
            Self {
                path: self.path.clone(),
                method: self.method.clone(),
                on_client: self.on_client.clone(),
            }
        }
    }

    impl<M: FnMarker> ServerFnStatic<M> {
        const fn path(&self) -> &str {
            self.path
        }
        const fn method(&self) -> Method {
            match self.method {
                Method::GET => Method::GET,
                Method::POST => Method::POST,
                Method::PUT => Method::PUT,
                Method::DELETE => Method::DELETE,
                Method::PATCH => Method::PATCH,
                Method::HEAD => Method::HEAD,
                Method::OPTIONS => Method::OPTIONS,
                _ => Method::GET,
            }
        }
    }

    struct ServerResponse<R> {
        _phant: std::marker::PhantomData<R>,
    }
    impl<R> ServerResponse<R> {
        fn new(f: impl Future<Output = R> + 'static) -> Self {
            Self {
                _phant: std::marker::PhantomData,
            }
        }
    }

    impl<R> Future for ServerResponse<R> {
        type Output = R;

        fn poll(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            todo!()
        }
    }

    trait FromResponse {}
    trait IntoRequest {}

    impl<M: FnMarker> Deref for ServerFnStatic<M>
    where
        M::Output: FromResponse,
    {
        type Target = M::UserCallable;

        fn deref(&self) -> &Self::Target {
            &self.on_client
        }
    }

    trait FnMarker {
        type Input;
        type Output;
        type UserCallable: Clone;
    }

    impl FnMarker for () {
        type Input = ();
        type Output = ();
        type UserCallable = fn() -> ServerResponse<()>;
    }

    impl<A, O> FnMarker for ((A,), O) {
        type Input = (A,);
        type Output = O;
        type UserCallable = fn(A) -> ServerResponse<O>;
    }

    impl<A, B, O> FnMarker for ((A, B), O) {
        type Input = (A, B);
        type Output = O;
        type UserCallable = fn(A, B) -> ServerResponse<O>;
    }

    impl<T: DeserializeOwned> FromResponse for anyhow::Result<T> {}
}
