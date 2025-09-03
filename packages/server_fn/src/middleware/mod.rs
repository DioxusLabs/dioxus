use crate::error::ServerFnErrorErr;
use bytes::Bytes;
use std::{future::Future, pin::Pin};

/// An abstraction over a middleware layer, which can be used to add additional
/// middleware layer to a [`Service`].
pub trait Layer<Req, Res>: Send + Sync + 'static {
    /// Adds this layer to the inner service.
    fn layer(&self, inner: BoxedService<Req, Res>) -> BoxedService<Req, Res>;
}

/// A type-erased service, which takes an HTTP request and returns a response.
pub struct BoxedService<Req, Res> {
    /// A function that converts a [`ServerFnErrorErr`] into a string.
    pub ser: fn(ServerFnErrorErr) -> Bytes,
    /// The inner service.
    pub service: Box<dyn Service<Req, Res> + Send>,
}

impl<Req, Res> BoxedService<Req, Res> {
    /// Constructs a type-erased service from this service.
    pub fn new(
        ser: fn(ServerFnErrorErr) -> Bytes,
        service: impl Service<Req, Res> + Send + 'static,
    ) -> Self {
        Self {
            ser,
            service: Box::new(service),
        }
    }

    /// Converts a request into a response by running the inner service.
    pub fn run(
        &mut self,
        req: Req,
    ) -> Pin<Box<dyn Future<Output = Res> + Send>> {
        self.service.run(req, self.ser)
    }
}

/// A service converts an HTTP request into a response.
pub trait Service<Request, Response> {
    /// Converts a request into a response.
    fn run(
        &mut self,
        req: Request,
        ser: fn(ServerFnErrorErr) -> Bytes,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}

#[cfg(feature = "axum-no-default")]
mod axum {
    use super::{BoxedService, Service};
    use crate::{error::ServerFnErrorErr, response::Res, ServerFnError};
    use axum::body::Body;
    use bytes::Bytes;
    use http::{Request, Response};
    use std::{future::Future, pin::Pin};

    impl<S> super::Service<Request<Body>, Response<Body>> for S
    where
        S: tower::Service<Request<Body>, Response = Response<Body>>,
        S::Future: Send + 'static,
        S::Error: std::fmt::Display + Send + 'static,
    {
        fn run(
            &mut self,
            req: Request<Body>,
            ser: fn(ServerFnErrorErr) -> Bytes,
        ) -> Pin<Box<dyn Future<Output = Response<Body>> + Send>> {
            let path = req.uri().path().to_string();
            let inner = self.call(req);
            Box::pin(async move {
                inner.await.unwrap_or_else(|e| {
                    let err =
                        ser(ServerFnErrorErr::MiddlewareError(e.to_string()));
                    Response::<Body>::error_response(&path, err)
                })
            })
        }
    }

    impl tower::Service<Request<Body>>
        for BoxedService<Request<Body>, Response<Body>>
    {
        type Response = Response<Body>;
        type Error = ServerFnError;
        type Future = Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<Self::Response, Self::Error>,
                    > + Send,
            >,
        >;

        fn poll_ready(
            &mut self,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            Ok(()).into()
        }

        fn call(&mut self, req: Request<Body>) -> Self::Future {
            let inner = self.service.run(req, self.ser);
            Box::pin(async move { Ok(inner.await) })
        }
    }

    impl<L> super::Layer<Request<Body>, Response<Body>> for L
    where
        L: tower_layer::Layer<BoxedService<Request<Body>, Response<Body>>>
            + Sync
            + Send
            + 'static,
        L::Service: Service<Request<Body>, Response<Body>> + Send + 'static,
    {
        fn layer(
            &self,
            inner: BoxedService<Request<Body>, Response<Body>>,
        ) -> BoxedService<Request<Body>, Response<Body>> {
            BoxedService::new(inner.ser, self.layer(inner))
        }
    }
}

#[cfg(feature = "actix-no-default")]
mod actix {
    use crate::{
        error::ServerFnErrorErr,
        request::actix::ActixRequest,
        response::{actix::ActixResponse, Res},
    };
    use actix_web::{HttpRequest, HttpResponse};
    use bytes::Bytes;
    use std::{future::Future, pin::Pin};

    impl<S> super::Service<HttpRequest, HttpResponse> for S
    where
        S: actix_web::dev::Service<HttpRequest, Response = HttpResponse>,
        S::Future: Send + 'static,
        S::Error: std::fmt::Display + Send + 'static,
    {
        fn run(
            &mut self,
            req: HttpRequest,
            ser: fn(ServerFnErrorErr) -> Bytes,
        ) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> {
            let path = req.uri().path().to_string();
            let inner = self.call(req);
            Box::pin(async move {
                inner.await.unwrap_or_else(|e| {
                    let err =
                        ser(ServerFnErrorErr::MiddlewareError(e.to_string()));
                    ActixResponse::error_response(&path, err).take()
                })
            })
        }
    }

    impl<S> super::Service<ActixRequest, ActixResponse> for S
    where
        S: actix_web::dev::Service<HttpRequest, Response = HttpResponse>,
        S::Future: Send + 'static,
        S::Error: std::fmt::Display + Send + 'static,
    {
        fn run(
            &mut self,
            req: ActixRequest,
            ser: fn(ServerFnErrorErr) -> Bytes,
        ) -> Pin<Box<dyn Future<Output = ActixResponse> + Send>> {
            let path = req.0 .0.uri().path().to_string();
            let inner = self.call(req.0.take().0);
            Box::pin(async move {
                ActixResponse::from(inner.await.unwrap_or_else(|e| {
                    let err =
                        ser(ServerFnErrorErr::MiddlewareError(e.to_string()));
                    ActixResponse::error_response(&path, err).take()
                }))
            })
        }
    }
}
