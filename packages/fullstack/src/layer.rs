use std::pin::Pin;
use tracing_futures::Instrument;

use http::{Request, Response};

pub type HttpBody = http_body_util::Full<bytes::Bytes>;

/// A layer that wraps a service. This can be used to add additional information to the request, or response on top of some other service
pub trait Layer: Send + Sync + 'static {
    /// Wrap a boxed service with this layer
    fn layer(&self, inner: BoxedService) -> BoxedService;
}

impl<L> Layer for L
where
    L: tower_layer::Layer<BoxedService> + Sync + Send + 'static,
    L::Service: tower::Service<http::Request<HttpBody>, Response = http::Response<HttpBody>>
        + Send
        + 'static,
        <<L as tower_layer::Layer<BoxedService>>::Service as tower::Service<
            http::Request<
                HttpBody
            >,
        >>::Error: std::error::Error,
        <<L as tower_layer::Layer<BoxedService>>::Service as tower::Service<
            http::Request<
                HttpBody
            >,
        >>::Future: Send,
{
    fn layer(&self, inner: BoxedService) -> BoxedService {
        BoxedService(Box::new(self.layer(inner)))
    }
}

/// A service is a function that takes a request and returns an async response
pub trait Service {
    /// Run the service and produce a future that resolves to a response
    fn run(
        &mut self,
        req: http::Request<HttpBody>,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<Response<HttpBody>, server_fn::ServerFnError>>
                + Send,
        >,
    >;
}

impl<S> Service for S
where
    S: tower::Service<http::Request<HttpBody>, Response = Response<HttpBody>>,
    S::Future: Send + 'static,
    S::Error: Into<server_fn::ServerFnError>,
{
    fn run(
        &mut self,
        req: http::Request<HttpBody>,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<Response<HttpBody>, server_fn::ServerFnError>>
                + Send,
        >,
    > {
        let fut = self.call(req).instrument(tracing::trace_span!(
            "service",
            "{}",
            std::any::type_name::<S>()
        ));
        Box::pin(async move { fut.await.map_err(|err| err.into()) })
    }
}

/// A boxed service is a type-erased service that can be used without knowing the underlying type
pub struct BoxedService(pub Box<dyn Service + Send>);

impl tower::Service<http::Request<HttpBody>> for BoxedService {
    type Response = http::Response<HttpBody>;
    type Error = server_fn::ServerFnError;
    type Future = Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<http::Response<HttpBody>, server_fn::ServerFnError>,
                > + Send,
        >,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<HttpBody>) -> Self::Future {
        Service::run(self, req)
    }
}
