use std::pin::Pin;
use tracing_futures::Instrument;

use http::{Request, Response};

pub trait Layer: Send + Sync + 'static {
    fn layer(&self, inner: BoxedService) -> BoxedService;
}

impl<L> Layer for L
where
    L: tower_layer::Layer<BoxedService> + Sync + Send + 'static,
    L::Service: Service + Send + 'static,
{
    fn layer(&self, inner: BoxedService) -> BoxedService {
        BoxedService(Box::new(self.layer(inner)))
    }
}

pub trait Service {
    fn run(
        &mut self,
        req: http::Request<hyper::body::Body>,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Response<hyper::body::Body>, server_fn::ServerFnError>,
                > + Send,
        >,
    >;
}

impl<S> Service for S
where
    S: tower::Service<http::Request<hyper::body::Body>, Response = Response<hyper::body::Body>>,
    S::Future: Send + 'static,
    S::Error: Into<server_fn::ServerFnError>,
{
    fn run(
        &mut self,
        req: http::Request<hyper::body::Body>,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Response<hyper::body::Body>, server_fn::ServerFnError>,
                > + Send,
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

pub struct BoxedService(pub Box<dyn Service + Send>);

impl tower::Service<http::Request<hyper::body::Body>> for BoxedService {
    type Response = http::Response<hyper::body::Body>;
    type Error = server_fn::ServerFnError;
    type Future = Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<http::Response<hyper::body::Body>, server_fn::ServerFnError>,
                > + Send,
        >,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<hyper::body::Body>) -> Self::Future {
        self.0.run(req)
    }
}
