use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    request::Req,
};
use axum::{
    body::{Body, Bytes},
    response::Response,
};
use futures::{Sink, Stream, StreamExt};
use http::{
    header::{ACCEPT, CONTENT_TYPE, REFERER},
    Request,
};
use http_body_util::BodyExt;
use std::borrow::Cow;

impl<Error, InputStreamError, OutputStreamError>
    Req<Error, InputStreamError, OutputStreamError> for Request<Body>
where
    Error: FromServerFnError + Send,
    InputStreamError: FromServerFnError + Send,
    OutputStreamError: FromServerFnError + Send,
{
    type WebsocketResponse = Response;

    fn as_query(&self) -> Option<&str> {
        self.uri().query()
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(CONTENT_TYPE)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(ACCEPT)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(REFERER)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    async fn try_into_bytes(self) -> Result<Bytes, Error> {
        let (_parts, body) = self.into_parts();

        body.collect().await.map(|c| c.to_bytes()).map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }

    async fn try_into_string(self) -> Result<String, Error> {
        let bytes = Req::<Error>::try_into_bytes(self).await?;
        String::from_utf8(bytes.to_vec()).map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, Error>
    {
        Ok(self.into_body().into_data_stream().map(|chunk| {
            chunk.map_err(|e| {
                Error::from_server_fn_error(ServerFnErrorErr::Deserialization(
                    e.to_string(),
                ))
                .ser()
            })
        }))
    }

    async fn try_into_websocket(
        self,
    ) -> Result<
        (
            impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
            impl Sink<Bytes> + Send + 'static,
            Self::WebsocketResponse,
        ),
        Error,
    > {
        #[cfg(not(feature = "axum"))]
        {
            Err::<
                (
                    futures::stream::Once<
                        std::future::Ready<Result<Bytes, Bytes>>,
                    >,
                    futures::sink::Drain<Bytes>,
                    Self::WebsocketResponse,
                ),
                Error,
            >(Error::from_server_fn_error(
                crate::ServerFnErrorErr::Response(
                    "Websocket connections not supported for Axum when the \
                     `axum` feature is not enabled on the `server_fn` crate."
                        .to_string(),
                ),
            ))
        }
        #[cfg(feature = "axum")]
        {
            use axum::extract::{ws::Message, FromRequest};
            use futures::FutureExt;

            let upgrade =
                axum::extract::ws::WebSocketUpgrade::from_request(self, &())
                    .await
                    .map_err(|err| {
                        Error::from_server_fn_error(ServerFnErrorErr::Request(
                            err.to_string(),
                        ))
                    })?;
            let (mut outgoing_tx, outgoing_rx) =
                futures::channel::mpsc::channel::<Result<Bytes, Bytes>>(2048);
            let (incoming_tx, mut incoming_rx) =
                futures::channel::mpsc::channel::<Bytes>(2048);
            let response = upgrade
        .on_failed_upgrade({
            let mut outgoing_tx = outgoing_tx.clone();
            move |err: axum::Error| {
                _ = outgoing_tx.start_send(Err(InputStreamError::from_server_fn_error(ServerFnErrorErr::Response(err.to_string())).ser()));
            }
        })
        .on_upgrade(|mut session| async move {
            loop {
                futures::select! {
                    incoming = incoming_rx.next() => {
                        let Some(incoming) = incoming else {
                            break;
                        };
                        if let Err(err) = session.send(Message::Binary(incoming)).await {
                            _ = outgoing_tx.start_send(Err(InputStreamError::from_server_fn_error(ServerFnErrorErr::Request(err.to_string())).ser()));
                        }
                    },
                        outgoing = session.recv().fuse() => {
                        let Some(outgoing) = outgoing else {
                            break;
                        };
                        match outgoing {
                            Ok(Message::Binary(bytes)) => {
                                _ = outgoing_tx
                                    .start_send(
                                        Ok(bytes),
                                    );
                            }
                            Ok(Message::Text(text)) => {
                                _ = outgoing_tx.start_send(Ok(Bytes::from(text)));
                            }
                            Ok(Message::Ping(bytes)) => {
                                if session.send(Message::Pong(bytes)).await.is_err() {
                                    break;
                                }
                            }
                            Ok(_other) => {}
                            Err(e) => {
                                _ = outgoing_tx.start_send(Err(InputStreamError::from_server_fn_error(ServerFnErrorErr::Response(e.to_string())).ser()));
                            }
                        }
                    }
                }
            }
            _ = session.send(Message::Close(None)).await;
        });

            Ok((outgoing_rx, incoming_tx, response))
        }
    }
}
